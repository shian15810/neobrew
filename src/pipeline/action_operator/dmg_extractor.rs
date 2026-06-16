use std::{
    path::{Component, Path, PathBuf},
    process::Stdio,
};

use anyhow::{Context as _, anyhow};
use async_trait::async_trait;
use async_walkdir::WalkDir;
use futures::{
    future::{self, TryFutureExt as _},
    stream::StreamExt as _,
};
use path_clean::PathClean as _;
use plist::Value;
use tempfile::{NamedTempFile, TempDir};
use tokio::{
    fs,
    io::{self, AsyncWriteExt as _},
    process::Command,
};

use super::{
    super::state_store::{ExtractedOutput, Stage, WrittenOutput},
    ActionOperator,
};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::prepared::{PreparedPackage, PreparedPackageExt as _, download::Download},
    util::archive_format::ArchiveFormat,
};

pub(crate) struct DmgExtractor;

#[async_trait]
impl ActionOperator for DmgExtractor {
    type Input = WrittenOutput;
    type Staging = (TempDir, PathBuf);
    type Output = ExtractedOutput;

    async fn should_run(
        &self,
        input: Option<&Self::Input>,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<bool> {
        let Some(input) = input else {
            return Ok(false);
        };

        let src_file_path = &input.dest_file_path;

        let download = prepared_package.download();

        let archive_format = download.archive_format();

        let is_dmg = self.is_dmg(src_file_path, archive_format).await?;

        Ok(is_dmg)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Extracting")
    }

    async fn execute(
        &self,
        input: Option<&Self::Input>,
        prepared_package: &PreparedPackage<Download>,
        context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let Some(input) = input else {
            let err = anyhow!("`Input` is supposed to be defined");

            return Err(err);
        };

        let src_file_path = &input.dest_file_path;

        let dest_dir_path = prepared_package.extract_dir_path(context);

        fs::create_dir_all(&dest_dir_path).await?;

        let src_dir = TempDir::new_in(&dest_dir_path)?;

        let src_dir_path = src_dir.path();

        self.extract(src_file_path, src_dir_path, &dest_dir_path)
            .await?;

        let staging = (src_dir, dest_dir_path);

        Ok(staging)
    }

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let (src_dir, dest_dir_path) = staging;

        src_dir.close()?;

        let output = ExtractedOutput {
            dest_dir_path,

            archive_format: ArchiveFormat::Dmg,
        };

        Ok(output)
    }

    fn passed_stage(
        &self,
        should_run: bool,
        _prepared_package: &PreparedPackage<Download>,
    ) -> Option<Stage> {
        should_run.then_some(Stage::Extracted)
    }
}

impl DmgExtractor {
    async fn is_dmg(
        &self,
        src_file_path: &Path,
        archive_format: Option<ArchiveFormat>,
    ) -> anyhow::Result<bool> {
        let is_dmg = match archive_format {
            Some(archive_format) => matches!(archive_format, ArchiveFormat::Dmg),
            None => ArchiveFormat::is_dmg(src_file_path).await?,
        };

        Ok(is_dmg)
    }

    async fn extract(
        &self,
        src_file_path: &Path,
        src_dir_path: &Path,
        dest_dir_path: &Path,
    ) -> anyhow::Result<()> {
        let mount_points = self.mount(src_file_path, src_dir_path).await?;

        if mount_points.is_empty() {
            let err = anyhow!("No mount point found in DMG");

            return Err(err);
        }

        let copy_mount_point_futs = mount_points
            .iter()
            .map(|mount_point| self.copy(mount_point, src_dir_path, dest_dir_path));

        let copy_mount_point_res = future::try_join_all(copy_mount_point_futs).await;

        let eject_mount_point_futs = mount_points
            .iter()
            .map(|mount_point| self.eject(mount_point));

        let eject_mount_point_res = future::join_all(eject_mount_point_futs).await;
        let eject_mount_point_res = eject_mount_point_res
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>();

        copy_mount_point_res.and(eject_mount_point_res)?;

        Ok(())
    }

    async fn mount(
        &self,
        src_file_path: &Path,
        src_dir_path: &Path,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let mount_points = self
            .mount_without_eula(src_file_path, src_dir_path)
            .or_else(|_| self.mount_with_eula(src_file_path, src_dir_path))
            .await?;

        Ok(mount_points)
    }

    async fn mount_without_eula(
        &self,
        src_file_path: &Path,
        src_dir_path: &Path,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let mut hdiutil = Command::new("hdiutil");

        hdiutil
            .arg("attach")
            .arg("-plist")
            .arg("-nobrowse")
            .arg("-readonly")
            .arg("-mountrandom")
            .arg(src_dir_path)
            .arg(src_file_path);

        hdiutil
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut hdiutil = hdiutil.spawn()?;

        if let Some(mut stdin) = hdiutil.stdin.take() {
            stdin.write_all(b"qn\n").await?;
        }

        let hdiutil = hdiutil.wait_with_output().await?;

        if !hdiutil.status.success() {
            let stdout = String::from_utf8_lossy(&hdiutil.stdout);

            let stderr = String::from_utf8_lossy(&hdiutil.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let mount_points = self.mount_points(&hdiutil.stdout)?;

        Ok(mount_points)
    }

    async fn mount_with_eula(
        &self,
        src_file_path: &Path,
        src_dir_path: &Path,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let dmg_file_stem = src_file_path
            .file_stem()
            .context("DMG path has no file stem")?;

        let cdr_path = src_dir_path.join(dmg_file_stem).with_added_extension("cdr");

        let mut hdiutil_convert = Command::new("hdiutil");

        hdiutil_convert
            .arg("convert")
            .arg("-quiet")
            .arg("-format")
            .arg("UDTO")
            .arg("-o")
            .arg(&cdr_path)
            .arg(src_file_path);

        let hdiutil_convert = hdiutil_convert.output().await?;

        if !hdiutil_convert.status.success() {
            let stdout = String::from_utf8_lossy(&hdiutil_convert.stdout);

            let stderr = String::from_utf8_lossy(&hdiutil_convert.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let mut hdiutil_attach = Command::new("hdiutil");

        hdiutil_attach
            .arg("attach")
            .arg("-plist")
            .arg("-nobrowse")
            .arg("-readonly")
            .arg("-mountrandom")
            .arg(src_dir_path)
            .arg(cdr_path);

        let hdiutil_attach = hdiutil_attach.output().await?;

        if !hdiutil_attach.status.success() {
            let stdout = String::from_utf8_lossy(&hdiutil_attach.stdout);

            let stderr = String::from_utf8_lossy(&hdiutil_attach.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let mount_points = self.mount_points(&hdiutil_attach.stdout)?;

        Ok(mount_points)
    }

    #[expect(clippy::unused_self)]
    fn mount_points(&self, stdout: &[u8]) -> anyhow::Result<Vec<PathBuf>> {
        let plist = plist::from_bytes::<Value>(stdout)?;

        let system_entities = plist
            .as_dictionary()
            .and_then(|dict| dict.get("system-entities"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();

        let mount_points = system_entities
            .iter()
            .filter_map(Value::as_dictionary)
            .filter_map(|dict| dict.get("mount-point"))
            .filter_map(Value::as_string)
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        Ok(mount_points)
    }

    async fn copy(
        &self,
        mount_point: &Path,
        src_dir_path: &Path,
        dest_dir_path: &Path,
    ) -> anyhow::Result<()> {
        let bom = self.bom(mount_point).await?;

        if bom.is_empty() {
            let err = anyhow!("No BOM found in mount point");

            return Err(err);
        }

        let list_file = NamedTempFile::new_in(src_dir_path)?;

        let list_file_path = list_file.path();

        fs::write(list_file_path, bom).await?;

        let bom_file = NamedTempFile::new_in(src_dir_path)?;

        let bom_file_path = bom_file.path();

        let mut mkbom = Command::new("mkbom");

        mkbom
            .arg("-s")
            .arg("-i")
            .arg(list_file_path)
            .arg("--")
            .arg(bom_file_path);

        let mkbom = mkbom.output().await?;

        list_file.close()?;

        if !mkbom.status.success() {
            let stdout = String::from_utf8_lossy(&mkbom.stdout);

            let stderr = String::from_utf8_lossy(&mkbom.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let mut ditto = Command::new("ditto");

        ditto
            .arg("--bom")
            .arg(bom_file_path)
            .arg("--")
            .arg(mount_point)
            .arg(dest_dir_path);

        let ditto = ditto.output().await?;

        bom_file.close()?;

        if !ditto.status.success() {
            let stdout = String::from_utf8_lossy(&ditto.stdout);

            let stderr = String::from_utf8_lossy(&ditto.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let mut dest_item_entries = WalkDir::new(dest_dir_path);

        while let Some(dest_item_entry) = dest_item_entries.next().await {
            let dest_item_entry = match dest_item_entry {
                Ok(dest_item_entry) => dest_item_entry,
                Err(err)
                    if err.io().map(io::Error::kind) == Some(io::ErrorKind::PermissionDenied) =>
                {
                    continue;
                },
                Err(err) => return Err(err)?,
            };

            let dest_item_path = dest_item_entry.path();

            if dest_item_path.starts_with(src_dir_path) {
                continue;
            }

            if dest_item_path.is_link_exists_nofollow().await? {
                continue;
            }

            dest_item_path.add_permissions_mode(0o200).await?;
        }

        Ok(())
    }

    async fn bom(&self, mount_point: &Path) -> anyhow::Result<String> {
        let mut mount_point_entry_relpstrs = Vec::new();

        mount_point_entry_relpstrs.push(".".to_owned());

        let mut mount_point_entries = WalkDir::new(mount_point);

        while let Some(mount_point_entry) = mount_point_entries.next().await {
            let mount_point_entry = match mount_point_entry {
                Ok(mount_point_entry) => mount_point_entry,
                Err(err)
                    if err.io().map(io::Error::kind) == Some(io::ErrorKind::PermissionDenied) =>
                {
                    continue;
                },
                Err(err) => return Err(err)?,
            };

            let mount_point_entry_path = mount_point_entry.path();

            if self.is_system_dir_link(&mount_point_entry_path).await? {
                continue;
            }

            let mount_point_entry_relpath = mount_point_entry_path.strip_prefix(mount_point)?;

            #[cfg(debug_assertions)]
            if mount_point_entry_relpath.is_empty() {
                continue;
            }

            #[cfg(not(debug_assertions))]
            if mount_point_entry_relpath.as_os_str().is_empty() {
                continue;
            }

            if self.is_dmg_metadata(mount_point_entry_relpath) {
                continue;
            }

            let mount_point_entry_relpstr = mount_point_entry_relpath.to_string_lossy();
            let mount_point_entry_relpstr = format!("./{mount_point_entry_relpstr}");

            mount_point_entry_relpstrs.push(mount_point_entry_relpstr);
        }

        mount_point_entry_relpstrs.sort();
        mount_point_entry_relpstrs.dedup();

        let mut bom = mount_point_entry_relpstrs.join("\n");

        bom.push('\n');

        Ok(bom)
    }

    async fn is_system_dir_link(&self, link_path: &Path) -> anyhow::Result<bool> {
        if !link_path.is_link_exists_nofollow().await? {
            return Ok(false);
        }

        let dir_path = fs::read_link(link_path).await?;
        let dir_path = if dir_path.is_relative() {
            let link_base_path = link_path.base()?;

            link_base_path.join(dir_path)
        } else {
            dir_path
        };
        let dir_path = dir_path.clean();

        let dir_pstr = dir_path.to_string_lossy();
        let dir_pstr = dir_pstr.as_ref();

        let is_system_dir_link = Self::SYSTEM_DIRS.contains(&dir_pstr);

        Ok(is_system_dir_link)
    }

    #[expect(clippy::unused_self)]
    fn is_dmg_metadata(&self, entry_relpath: &Path) -> bool {
        let is_dmg_metadata = entry_relpath
            .components()
            .find_map(|component| match component {
                Component::Normal(first_component_pstr) => first_component_pstr.to_str(),
                _ => None,
            })
            .map(|first_component_pstr| Self::DMG_METADATA.contains(&first_component_pstr));

        is_dmg_metadata == Some(true)
    }

    async fn eject(&self, mount_point: &Path) -> anyhow::Result<()> {
        if !mount_point.try_exists()? {
            return Ok(());
        }

        self.eject_apfs_hfs(mount_point)
            .or_else(|_| self.eject_force(mount_point))
            .await?;

        Ok(())
    }

    async fn eject_apfs_hfs(&self, mount_point: &Path) -> anyhow::Result<()> {
        let mut diskutil_info = Command::new("diskutil");

        diskutil_info.arg("info").arg("-plist").arg(mount_point);

        let diskutil_info = diskutil_info.output().await?;

        if !diskutil_info.status.success() {
            let stdout = String::from_utf8_lossy(&diskutil_info.stdout);

            let stderr = String::from_utf8_lossy(&diskutil_info.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let plist = plist::from_bytes::<Value>(&diskutil_info.stdout)?;

        let apfs_physical_stores = plist
            .as_dictionary()
            .and_then(|dict| dict.get("APFSPhysicalStores"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();

        let apfs_physical_stores = apfs_physical_stores
            .iter()
            .filter_map(Value::as_dictionary)
            .filter_map(|dict| dict.get("APFSPhysicalStore"))
            .filter_map(Value::as_string)
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        let eject_paths = if apfs_physical_stores.is_empty() {
            vec![mount_point.to_owned()]
        } else {
            apfs_physical_stores
        };

        let eject_path_futs = eject_paths.into_iter().map(async |eject_path| {
            let mut diskutil_eject = Command::new("diskutil");

            diskutil_eject.arg("eject").arg(eject_path);

            let diskutil_eject = diskutil_eject.output().await?;

            if !diskutil_eject.status.success() {
                let stdout = String::from_utf8_lossy(&diskutil_eject.stdout);

                let stderr = String::from_utf8_lossy(&diskutil_eject.stderr);

                let err = anyhow!("{stdout}{stderr}");

                return Err(err);
            }

            anyhow::Ok(())
        });

        let eject_path_res = future::join_all(eject_path_futs).await;
        let eject_path_res = eject_path_res
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>();

        eject_path_res?;

        Ok(())
    }

    async fn eject_force(&self, mount_point: &Path) -> anyhow::Result<()> {
        let mut diskutil = Command::new("diskutil");

        diskutil.arg("unmount").arg("force").arg(mount_point);

        let diskutil = diskutil.output().await?;

        if !diskutil.status.success() {
            let stdout = String::from_utf8_lossy(&diskutil.stdout);

            let stderr = String::from_utf8_lossy(&diskutil.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        Ok(())
    }

    const SYSTEM_DIRS: &[&str] = &[
        "/",
        "/Applications",
        "/Applications/Utilities",
        "/Incompatible Software",
        "/Library",
        "/Library/Application Support",
        "/Library/Audio",
        "/Library/Caches",
        "/Library/ColorPickers",
        "/Library/ColorSync",
        "/Library/Components",
        "/Library/Compositions",
        "/Library/Contextual Menu Items",
        "/Library/CoreMediaIO",
        "/Library/Desktop Pictures",
        "/Library/Developer",
        "/Library/Dictionaries",
        "/Library/DirectoryServices",
        "/Library/Documentation",
        "/Library/Extensions",
        "/Library/Filesystems",
        "/Library/Fonts",
        "/Library/Frameworks",
        "/Library/Graphics",
        "/Library/Image Capture",
        "/Library/Input Methods",
        "/Library/Internet Plug-Ins",
        "/Library/Java",
        "/Library/Java/Extensions",
        "/Library/Java/JavaVirtualMachines",
        "/Library/Keyboard Layouts",
        "/Library/Keychains",
        "/Library/LaunchAgents",
        "/Library/LaunchDaemons",
        "/Library/Logs",
        "/Library/Messages",
        "/Library/Modem Scripts",
        "/Library/OpenDirectory",
        "/Library/PDF Services",
        "/Library/Perl",
        "/Library/PreferencePanes",
        "/Library/Preferences",
        "/Library/Printers",
        "/Library/PrivilegedHelperTools",
        "/Library/Python",
        "/Library/QuickLook",
        "/Library/QuickTime",
        "/Library/Receipts",
        "/Library/Ruby",
        "/Library/Sandbox",
        "/Library/Screen Savers",
        "/Library/ScriptingAdditions",
        "/Library/Scripts",
        "/Library/Security",
        "/Library/Speech",
        "/Library/Spelling",
        "/Library/Spotlight",
        "/Library/StartupItems",
        "/Library/SystemProfiler",
        "/Library/Updates",
        "/Library/User Pictures",
        "/Library/Video",
        "/Library/WebServer",
        "/Library/Widgets",
        "/Library/iTunes",
        "/Network",
        "/System",
        "/System/Library",
        "/System/Library/Accessibility",
        "/System/Library/Accounts",
        "/System/Library/Address Book Plug-Ins",
        "/System/Library/Toolbox",
        "/System/Library/Automator",
        "/System/Library/BridgeSupport",
        "/System/Library/Caches",
        "/System/Library/ColorPickers",
        "/System/Library/ColorSync",
        "/System/Library/Colors",
        "/System/Library/Components",
        "/System/Library/Compositions",
        "/System/Library/CoreServices",
        "/System/Library/DTDs",
        "/System/Library/DirectoryServices",
        "/System/Library/Displays",
        "/System/Library/Extensions",
        "/System/Library/Filesystems",
        "/System/Library/Filters",
        "/System/Library/Fonts",
        "/System/Library/Frameworks",
        "/System/Library/Graphics",
        "/System/Library/IdentityServices",
        "/System/Library/Image Capture",
        "/System/Library/Input Methods",
        "/System/Library/InternetAccounts",
        "/System/Library/Java",
        "/System/Library/KerberosPlugins",
        "/System/Library/Keyboard Layouts",
        "/System/Library/Keychains",
        "/System/Library/LaunchAgents",
        "/System/Library/LaunchDaemons",
        "/System/Library/LinguisticData",
        "/System/Library/LocationBundles",
        "/System/Library/LoginPlugins",
        "/System/Library/Messages",
        "/System/Library/Metadata",
        "/System/Library/MonitorPanels",
        "/System/Library/OpenDirectory",
        "/System/Library/OpenSSL",
        "/System/Library/Password Server Filters",
        "/System/Library/PerformanceMetrics",
        "/System/Library/Perl",
        "/System/Library/PreferencePanes",
        "/System/Library/Printers",
        "/System/Library/PrivateFrameworks",
        "/System/Library/QuickLook",
        "/System/Library/QuickTime",
        "/System/Library/QuickTimeJava",
        "/System/Library/Recents",
        "/System/Library/SDKSettingsPlist",
        "/System/Library/Sandbox",
        "/System/Library/Screen Savers",
        "/System/Library/ScreenReader",
        "/System/Library/ScriptingAdditions",
        "/System/Library/ScriptingDefinitions",
        "/System/Library/Security",
        "/System/Library/Services",
        "/System/Library/Sounds",
        "/System/Library/Speech",
        "/System/Library/Spelling",
        "/System/Library/Spotlight",
        "/System/Library/StartupItems",
        "/System/Library/SyncServices",
        "/System/Library/SystemConfiguration",
        "/System/Library/SystemProfiler",
        "/System/Library/Tcl",
        "/System/Library/TextEncodings",
        "/System/Library/User Template",
        "/System/Library/UserEventPlugins",
        "/System/Library/Video",
        "/System/Library/WidgetResources",
        "/User Information",
        "/Users",
        "/Volumes",
        "/bin",
        "/boot",
        "/cores",
        "/dev",
        "/etc",
        "/etc/X11",
        "/etc/opt",
        "/etc/sgml",
        "/etc/xml",
        "/home",
        "/libexec",
        "/lost+found",
        "/media",
        "/mnt",
        "/net",
        "/opt",
        "/private",
        "/private/etc",
        "/private/tftpboot",
        "/private/tmp",
        "/private/var",
        "/proc",
        "/root",
        "/sbin",
        "/srv",
        "/tmp",
        "/usr",
        "/usr/X11R6",
        "/usr/bin",
        "/usr/etc",
        "/usr/include",
        "/usr/lib",
        "/usr/libexec",
        "/usr/libexec/cups",
        "/usr/local",
        "/usr/local/Cellar",
        "/usr/local/Frameworks",
        "/usr/local/Library",
        "/usr/local/bin",
        "/usr/local/etc",
        "/usr/local/include",
        "/usr/local/lib",
        "/usr/local/libexec",
        "/usr/local/opt",
        "/usr/local/share",
        "/usr/local/share/man",
        "/usr/local/share/man/man1",
        "/usr/local/share/man/man2",
        "/usr/local/share/man/man3",
        "/usr/local/share/man/man4",
        "/usr/local/share/man/man5",
        "/usr/local/share/man/man6",
        "/usr/local/share/man/man7",
        "/usr/local/share/man/man8",
        "/usr/local/share/man/man9",
        "/usr/local/share/man/mann",
        "/usr/local/var",
        "/usr/local/var/lib",
        "/usr/local/var/lock",
        "/usr/local/var/run",
        "/usr/sbin",
        "/usr/share",
        "/usr/share/man",
        "/usr/share/man/man1",
        "/usr/share/man/man2",
        "/usr/share/man/man3",
        "/usr/share/man/man4",
        "/usr/share/man/man5",
        "/usr/share/man/man6",
        "/usr/share/man/man7",
        "/usr/share/man/man8",
        "/usr/share/man/man9",
        "/usr/share/man/mann",
        "/usr/src",
        "/var",
        "/var/cache",
        "/var/lib",
        "/var/lock",
        "/var/log",
        "/var/mail",
        "/var/run",
        "/var/spool",
        "/var/spool/mail",
        "/var/tmp",
    ];

    const DMG_METADATA: &[&str] = &[
        ".background",
        ".com.apple.timemachine.donotpresent",
        ".com.apple.timemachine.supported",
        ".DocumentRevisions-V100",
        ".DS_Store",
        ".fseventsd",
        ".MobileBackups",
        ".Spotlight-V100",
        ".TemporaryItems",
        ".Trashes",
        ".VolumeIcon.icns",
        ".HFS+ Private Directory Data\r",
        ".HFS+ Private Data\r",
    ];
}
