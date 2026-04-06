mod language_servers;

use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree, serde_json::Value};

use crate::language_servers::{Taplo, Tombi};

struct TomlExtension {
    taplo: Option<Taplo>,
    tombi: Option<Tombi>,
}

impl zed::Extension for TomlExtension {
    fn new() -> Self {
        Self {
            taplo: None,
            tombi: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            Taplo::LANGUAGE_SERVER_ID => self
                .taplo
                .get_or_insert_with(Taplo::new)
                .language_server_command(language_server_id, worktree),
            Tombi::LANGUAGE_SERVER_ID => self
                .tombi
                .get_or_insert_with(Tombi::new)
                .language_server_command(language_server_id, worktree),
            language_server_id => Err(format!("unknown language server: {language_server_id}")),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        match language_server_id.as_ref() {
            Taplo::LANGUAGE_SERVER_ID => self
                .taplo
                .get_or_insert_with(Taplo::new)
                .language_server_initialization_options(worktree),
            Tombi::LANGUAGE_SERVER_ID => self
                .tombi
                .get_or_insert_with(Tombi::new)
                .language_server_initialization_options(worktree),
            _ => Ok(None),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        match language_server_id.as_ref() {
            Taplo::LANGUAGE_SERVER_ID => self
                .taplo
                .get_or_insert_with(Taplo::new)
                .language_server_workspace_configuration(worktree),
            Tombi::LANGUAGE_SERVER_ID => self
                .tombi
                .get_or_insert_with(Tombi::new)
                .language_server_workspace_configuration(worktree),
            _ => Ok(None),
        }
    }
}

zed::register_extension!(TomlExtension);
