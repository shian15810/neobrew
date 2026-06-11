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
use tokio::{fs, io::AsyncWriteExt as _, process::Command};

use super::{
    super::state_store::{PouredOutput, Stage, WrittenOutput},
    ActionOperator,
};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::prepared::PreparedPackage,
    util::ArchiveFormat,
};

pub(crate) struct DmgPourer {
    temp_dir: TempDir,

    dest_dir_path: PathBuf,
    archive_format: Option<ArchiveFormat>,
}

#[async_trait]
impl ActionOperator for DmgPourer {
    type Input = WrittenOutput;
    type Staging = ();
    type Output = PouredOutput;

    async fn should_run(
        &self,
        input: Option<&Self::Input>,
        _prepared_package: &PreparedPackage,
    ) -> anyhow::Result<bool> {
        let Some(input) = input else {
            return Ok(false);
        };

        let is_dmg = self.is_dmg(&input.dest_file_path).await?;

        Ok(is_dmg)
    }

    fn on_skip_run(self) -> anyhow::Result<Option<Self::Output>> {
        self.cleanup()?;

        Ok(None)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Pouring")
    }

    async fn execute(
        &self,
        input: Option<&Self::Input>,
        _prepared_package: &PreparedPackage,
        _context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let Some(input) = input else {
            let err = anyhow!("`Input` is supposed to be defined");

            return Err(err);
        };

        self.pour(&input.dest_file_path).await?;

        Ok(())
    }

    fn on_final_run(self, _staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let dest_dir_path = self.dest_dir_path.clone();

        self.cleanup()?;

        let output = PouredOutput {
            dest_dir_path,
            archive_format: ArchiveFormat::Dmg,
        };

        Ok(output)
    }

    fn cleanup(self) -> anyhow::Result<()> {
        self.temp_dir.close()?;

        Ok(())
    }

    fn passed_stage(&self, should_run: bool, _prepared_package: &PreparedPackage) -> Option<Stage> {
        should_run.then_some(Stage::Poured)
    }
}

impl DmgPourer {
    pub(crate) async fn try_init(
        dest_dir_path: PathBuf,
        archive_format: Option<ArchiveFormat>,
    ) -> anyhow::Result<Self> {
        let dest_base_path = &dest_dir_path;

        fs::create_dir_all(dest_base_path).await?;

        let temp_dir = TempDir::new_in(dest_base_path)?;

        let this = Self {
            temp_dir,

            dest_dir_path,
            archive_format,
        };

        Ok(this)
    }

    async fn is_dmg(&self, src_file_path: &Path) -> anyhow::Result<bool> {
        let is_dmg = if let Some(archive_format) = &self.archive_format {
            matches!(archive_format, ArchiveFormat::Dmg)
        } else {
            ArchiveFormat::is_dmg(src_file_path).await?
        };

        Ok(is_dmg)
    }

    async fn pour(&self, src_file_path: &Path) -> anyhow::Result<()> {
        let mount_points = self.mount(src_file_path).await?;

        if mount_points.is_empty() {
            let err = anyhow!("No mount point found in DMG");

            return Err(err);
        }

        let extract_mount_point_futs = mount_points
            .iter()
            .map(|mount_point| self.extract(mount_point));

        let extract_mount_point_res = future::try_join_all(extract_mount_point_futs).await;

        let eject_mount_point_futs = mount_points
            .iter()
            .map(|mount_point| self.eject(mount_point));

        let eject_mount_point_res = future::join_all(eject_mount_point_futs).await;
        let eject_mount_point_res = eject_mount_point_res
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>();

        extract_mount_point_res.and(eject_mount_point_res)?;

        Ok(())
    }

    async fn mount(&self, src_file_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let mount_points = self
            .mount_without_eula(src_file_path)
            .or_else(|_| self.mount_with_eula(src_file_path))
            .await?;

        Ok(mount_points)
    }

    async fn mount_without_eula(&self, src_file_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let src_base_path = self.temp_dir.path();

        let mut hdiutil = Command::new("hdiutil");

        hdiutil
            .arg("attach")
            .arg("-plist")
            .arg("-nobrowse")
            .arg("-readonly")
            .arg("-mountrandom")
            .arg(src_base_path)
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
            let stderr = String::from_utf8_lossy(&hdiutil.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

            return Err(err);
        }

        let mount_points = self.mount_points(&hdiutil.stdout)?;

        Ok(mount_points)
    }

    async fn mount_with_eula(&self, src_file_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let src_base_path = self.temp_dir.path();

        let dmg_file_stem = src_file_path
            .file_stem()
            .context("DMG path has no file stem")?;

        let cdr_path = src_base_path
            .join(dmg_file_stem)
            .with_added_extension("cdr");

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
            let stderr = String::from_utf8_lossy(&hdiutil_convert.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

            return Err(err);
        }

        let mut hdiutil_attach = Command::new("hdiutil");

        hdiutil_attach
            .arg("attach")
            .arg("-plist")
            .arg("-nobrowse")
            .arg("-readonly")
            .arg("-mountrandom")
            .arg(src_base_path)
            .arg(cdr_path);

        let hdiutil_attach = hdiutil_attach.output().await?;

        if !hdiutil_attach.status.success() {
            let stderr = String::from_utf8_lossy(&hdiutil_attach.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

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

    async fn extract(&self, mount_point: &Path) -> anyhow::Result<()> {
        let src_base_path = self.temp_dir.path();

        let dest_base_path = &self.dest_dir_path;

        let bom = self.bom(mount_point).await?;

        if bom.is_empty() {
            let err = anyhow!("No BOM found in mount point");

            return Err(err);
        }

        let temp_list_file = NamedTempFile::new_in(dest_base_path)?;

        let temp_bom_file = NamedTempFile::new_in(dest_base_path)?;

        let temp_list_file_path = temp_list_file.path();

        let temp_bom_file_path = temp_bom_file.path();

        fs::write(temp_list_file_path, bom).await?;

        let mut mkbom = Command::new("mkbom");

        mkbom
            .arg("-s")
            .arg("-i")
            .arg(temp_list_file_path)
            .arg("--")
            .arg(temp_bom_file_path);

        let mkbom = mkbom.output().await?;

        if !mkbom.status.success() {
            let stderr = String::from_utf8_lossy(&mkbom.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

            return Err(err);
        }

        let mut ditto = Command::new("ditto");

        ditto
            .arg("--bom")
            .arg(temp_bom_file_path)
            .arg("--")
            .arg(mount_point)
            .arg(dest_base_path);

        let ditto = ditto.output().await?;

        if !ditto.status.success() {
            let stderr = String::from_utf8_lossy(&ditto.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

            return Err(err);
        }

        temp_bom_file.close()?;

        temp_list_file.close()?;

        let mut dest_item_entries = WalkDir::new(dest_base_path);

        while let Some(dest_item_entry) = dest_item_entries.next().await {
            let dest_item_entry = dest_item_entry?;

            let dest_item_path = dest_item_entry.path();

            if dest_item_path.starts_with(src_base_path) {
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
            let mount_point_entry = mount_point_entry?;

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

    async fn is_system_dir_link(&self, path: &Path) -> anyhow::Result<bool> {
        if !path.is_link_exists_nofollow().await? {
            return Ok(false);
        }

        let link_path = fs::read_link(path).await?;
        let link_path = if link_path.is_relative() {
            let link_base_path = path.base()?;

            link_base_path.join(link_path)
        } else {
            link_path
        };
        let link_path = link_path.clean();

        let link_pstr = link_path.to_string_lossy();
        let link_pstr = link_pstr.as_ref();

        let is_system_dir_link = Self::SYSTEM_DIRS.contains(&link_pstr);

        Ok(is_system_dir_link)
    }

    #[expect(clippy::unused_self)]
    fn is_dmg_metadata(&self, entry_relpath: &Path) -> bool {
        entry_relpath
            .components()
            .find_map(|component| match component {
                Component::Normal(first_component_pstr) => first_component_pstr.to_str(),
                _ => None,
            })
            .is_some_and(|first_component_pstr| Self::DMG_METADATA.contains(&first_component_pstr))
    }

    async fn eject(&self, mount_point: &Path) -> anyhow::Result<()> {
        if !mount_point.exists() {
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
            let stderr = String::from_utf8_lossy(&diskutil_info.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

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
                let stderr = String::from_utf8_lossy(&diskutil_eject.stderr);
                let stderr = stderr.into_owned();

                let err = anyhow!(stderr);

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
        let mut diskutil_unmount = Command::new("diskutil");

        diskutil_unmount
            .arg("unmount")
            .arg("force")
            .arg(mount_point);

        let diskutil_unmount = diskutil_unmount.output().await?;

        if !diskutil_unmount.status.success() {
            let stderr = String::from_utf8_lossy(&diskutil_unmount.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

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
