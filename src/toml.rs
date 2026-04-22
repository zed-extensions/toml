mod config;
mod util;

use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree, serde_json::Value};

struct TaploBinary {
    path: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

struct TomlExtension {
    cached_binary_path: Option<String>,
}

impl TomlExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<TaploBinary> {
        let (platform, arch) = zed::current_platform();

        let binary_settings = config::get_binary_settings(language_server_id.as_ref(), worktree);
        let binary_args = config::get_binary_args(&binary_settings)
            .unwrap_or_else(|| vec!["lsp".to_string(), "stdio".to_string()]);
        let binary_env = config::get_binary_env(&binary_settings).unwrap_or_default();

        if let Some(binary_path) = config::get_binary_path(&binary_settings) {
            return Ok(TaploBinary {
                path: binary_path,
                args: binary_args,
                env: binary_env,
            });
        }

        if let Some(binary_path) = worktree.which(language_server_id.as_ref()) {
            return Ok(TaploBinary {
                path: binary_path,
                args: binary_args,
                env: binary_env,
            });
        }

        if let Some(binary_path) = &self.cached_binary_path
            && fs::metadata(binary_path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(TaploBinary {
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
            "tamasfe/taplo",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        ) {
            Ok(release) => release,
            Err(_) => {
                if let Some(binary_path) = util::find_existing_binary(
                    language_server_id.as_ref(),
                    language_server_id.as_ref(),
                ) {
                    self.cached_binary_path = Some(binary_path.clone());
                    return Ok(TaploBinary {
                        path: binary_path,
                        args: binary_args,
                        env: binary_env,
                    });
                }
                return Err("failed to download latest github release".to_string());
            }
        };

        let asset_name = format!(
            "{}-{os}-{arch}.gz",
            language_server_id.as_ref(),
            os = match platform {
                zed::Os::Mac => "darwin",
                zed::Os::Linux => "linux",
                zed::Os::Windows => "windows",
            },
            arch = match arch {
                zed::Architecture::Aarch64 => "aarch64",
                zed::Architecture::X8664 => "x86_64",
                zed::Architecture::X86 =>
                    return Err(format!("unsupported architecture: {arch:?}")),
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("{}-{}", language_server_id.as_ref(), release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

        let binary_path = format!("{}/{}", version_dir, language_server_id.as_ref());

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Gzip,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            util::remove_outdated_versions(language_server_id.as_ref(), &version_dir)?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(TaploBinary {
            path: binary_path,
            args: binary_args,
            env: binary_env,
        })
    }
}

impl zed::Extension for TomlExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let taplo = self.language_server_binary(language_server_id, worktree)?;

        Ok(zed::Command {
            command: taplo.path,
            args: taplo.args,
            env: taplo.env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings = config::get_initialization_options(language_server_id.as_ref(), worktree)
            .unwrap_or_default();

        Ok(Some(settings))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings = config::get_workspace_configuration(language_server_id.as_ref(), worktree)
            .unwrap_or_default();

        Ok(Some(settings))
    }
}

zed::register_extension!(TomlExtension);
