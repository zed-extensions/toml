use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree, serde_json::Value};

use crate::language_servers::{config, util};

struct TombiBinary {
    path: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

pub struct Tombi {
    cached_binary_path: Option<String>,
}

impl Tombi {
    pub const LANGUAGE_SERVER_ID: &'static str = "tombi";

    pub fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    pub fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let tombi = self.language_server_binary(language_server_id, worktree)?;

        Ok(zed::Command {
            command: tombi.path,
            args: tombi.args,
            env: tombi.env,
        })
    }

    fn language_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<TombiBinary> {
        let (platform, arch) = zed::current_platform();
        let extension = match platform {
            zed::Os::Mac | zed::Os::Linux => "",
            zed::Os::Windows => ".exe",
        };

        let binary_name = format!("{}{extension}", Self::LANGUAGE_SERVER_ID);
        let binary_settings = config::get_binary_settings(Self::LANGUAGE_SERVER_ID, worktree);
        let binary_args =
            config::get_binary_args(&binary_settings).unwrap_or_else(|| vec!["lsp".to_string()]);
        let binary_env = config::get_binary_env(&binary_settings).unwrap_or_default();

        if let Some(binary_path) = config::get_binary_path(&binary_settings) {
            return Ok(TombiBinary {
                path: binary_path,
                args: binary_args,
                env: binary_env,
            });
        }

        if let Some(binary_path) = worktree.which(Self::LANGUAGE_SERVER_ID) {
            return Ok(TombiBinary {
                path: binary_path,
                args: binary_args,
                env: binary_env,
            });
        }

        if let Some(binary_path) = &self.cached_binary_path
            && fs::metadata(binary_path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(TombiBinary {
                path: binary_path.clone(),
                args: binary_args,
                env: binary_env,
            });
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = match zed::latest_github_release(
            "tombi-toml/tombi",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        ) {
            Ok(release) => release,
            Err(_) => {
                if let Some(binary_path) =
                    util::find_existing_binary(Self::LANGUAGE_SERVER_ID, &binary_name)
                {
                    self.cached_binary_path = Some(binary_path.clone());
                    return Ok(TombiBinary {
                        path: binary_path,
                        args: binary_args,
                        env: binary_env,
                    });
                }
                return Err("failed to download latest github release".to_string());
            }
        };

        let asset_name = format!(
            "{}-cli-{version}-{arch}-{os}{extension}",
            Self::LANGUAGE_SERVER_ID,
            version = release
                .version
                .strip_prefix("v")
                .unwrap_or(&release.version),
            arch = match arch {
                zed::Architecture::Aarch64 => "aarch64",
                zed::Architecture::X8664 => "x86_64",
                zed::Architecture::X86 =>
                    return Err(format!("unsupported architecture: {arch:?}")),
            },
            os = match platform {
                zed::Os::Mac => "apple-darwin",
                zed::Os::Linux => "unknown-linux-musl",
                zed::Os::Windows => "pc-windows-msvc",
            },
            extension = match platform {
                zed::Os::Mac | zed::Os::Linux => ".gz",
                zed::Os::Windows => ".zip",
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("{}-{}", Self::LANGUAGE_SERVER_ID, release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

        let binary_path = format!("{}/{}", version_dir, binary_name);

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let (file_path, file_type) = match platform {
                zed::Os::Mac | zed::Os::Linux => (&binary_path, zed::DownloadedFileType::Gzip),
                zed::Os::Windows => (&version_dir, zed::DownloadedFileType::Zip),
            };

            zed::download_file(&asset.download_url, file_path, file_type)
                .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            util::remove_outdated_versions(Self::LANGUAGE_SERVER_ID, &version_dir)?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(TombiBinary {
            path: binary_path,
            args: binary_args,
            env: binary_env,
        })
    }

    pub fn language_server_initialization_options(
        &mut self,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings = config::get_initialization_options(Self::LANGUAGE_SERVER_ID, worktree)
            .unwrap_or_default();

        Ok(Some(settings))
    }

    pub fn language_server_workspace_configuration(
        &mut self,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings = config::get_workspace_configuration(Self::LANGUAGE_SERVER_ID, worktree)
            .unwrap_or_default();

        Ok(Some(settings))
    }
}
