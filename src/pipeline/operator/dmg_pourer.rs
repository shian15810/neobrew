use std::path::{Path, PathBuf};

use anyhow::anyhow;
use plist::Value;
use tempfile::TempDir;
use tokio::{fs, process::Command};

struct DmgPourer {
    temp_dir: TempDir,

    dmg_file_path: PathBuf,
    dest_dir_path: PathBuf,
}

impl DmgPourer {
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

    async fn try_init(dmg_file_path: PathBuf, dest_dir_path: PathBuf) -> anyhow::Result<Self> {
        let dest_base_path = &dest_dir_path;

        fs::create_dir_all(dest_base_path).await?;

        let temp_dir = TempDir::new_in(dest_base_path)?;

        let this = Self {
            temp_dir,

            dmg_file_path,
            dest_dir_path,
        };

        Ok(this)
    }

    async fn pour(self) -> anyhow::Result<()> {
        let mounts = self.attach().await?;

        self.pour_mounts(&mounts);

        Ok(())
    }

    async fn attach(&self) -> anyhow::Result<Vec<PathBuf>> {
        let src_base_path = self.temp_dir.path();

        let dmg_file_path = &self.dmg_file_path;

        let mut hdiutil = Command::new("hdiutil");

        hdiutil
            .arg("attach")
            .arg("-plist")
            .arg("-nobrowse")
            .arg("-readonly")
            .arg("-mountrandom")
            .arg(src_base_path)
            .arg(dmg_file_path);

        #[cfg(not(debug_assertions))]
        hdiutil
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let output = hdiutil.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.into_owned();

            let err = anyhow!(stderr);

            return Err(err);
        }

        let plist = plist::from_bytes::<Value>(&output.stdout)?;

        let entities = plist
            .as_dictionary()
            .and_then(|dict| dict.get("system-entities"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();

        let mounts = entities
            .iter()
            .filter_map(Value::as_dictionary)
            .filter_map(|dict| dict.get("mount-point"))
            .filter_map(Value::as_string)
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        if mounts.is_empty() {
            let err = anyhow!("No mounts found in disk image");

            return Err(err);
        }

        Ok(mounts)
    }

    fn pour_mounts(&self, mounts: &[PathBuf]) {
        for mount in mounts {
            self.pour_mount(mount);
        }
    }

    fn pour_mount(&self, _mount: &Path) {
        let dest_base_path = &self.dest_dir_path;

        let mut ditto = Command::new("ditto");

        ditto.arg(dest_base_path);
    }
}
