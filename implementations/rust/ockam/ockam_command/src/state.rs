use anyhow::Context;
use ockam_identity::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use ockam_identity::{Identity, IdentityIdentifier};
use ockam_vault::{storage::FileStorage, Vault};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct CliState {
    pub vaults: VaultsState,
    pub identities: IdentitiesState,
    pub nodes: NodesState,
    dir: PathBuf,
}

impl CliState {
    pub fn new() -> anyhow::Result<Self> {
        let dir = Self::dir()?;
        Ok(Self {
            vaults: VaultsState::new(&dir)?,
            identities: IdentitiesState::new(&dir)?,
            nodes: NodesState::new(&dir)?,
            dir,
        })
    }

    fn dir() -> anyhow::Result<PathBuf> {
        Ok(match std::env::var("OCKAM_HOME") {
            Ok(dir) => PathBuf::from(&dir),
            Err(_) => dirs::home_dir()
                .context("no $HOME directory")?
                .join(".ockam"),
        })
    }

    pub fn create_node(&self, name: &str, config: NodeConfig) -> anyhow::Result<NodeState> {
        let vault = match &config.vault {
            Some(vault) => self.vaults.get(vault)?,
            None => self.vaults.default()?,
        };
        let identity = match &config.identity {
            Some(identity) => self.identities.get(identity)?,
            None => self.identities.default()?,
        };
        self.nodes.create(vault, identity, name)
    }

    pub fn node(&self, name: &str) -> anyhow::Result<NodeState> {
        let vault = {
            let name = self.nodes.vault_name(name)?;
            self.vaults.get(&name)?
        };
        let identity = {
            let name = self.nodes.identity_name(name)?;
            self.identities.get(&name)?
        };
        self.nodes.get(vault, identity, name)
    }
}

pub struct VaultsState {
    dir: PathBuf,
}

impl VaultsState {
    fn new(cli_path: &Path) -> anyhow::Result<Self> {
        let dir = cli_path.join("vaults");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn create(&self, name: &str, config: VaultConfig) -> anyhow::Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        Ok(VaultState { path, config })
    }

    pub fn get(&self, name: &str) -> anyhow::Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if !path.exists() {
                return Err(anyhow::anyhow!("vault `{name}` does not exist"));
            }
            path
        };
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(VaultState { path, config })
    }

    pub fn default(&self) -> anyhow::Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push("default");
            std::fs::canonicalize(&path)?
        };
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(VaultState { path, config })
    }

    pub fn set_default(&self, name: &str) -> anyhow::Result<VaultState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            path
        };
        let link = {
            let mut path = self.dir.clone();
            path.push("default");
            path
        };
        std::os::unix::fs::symlink(&original, &link)?;
        let contents = std::fs::read_to_string(&original)?;
        let config = serde_json::from_str(&contents)?;
        Ok(VaultState {
            path: original,
            config,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultState {
    path: PathBuf,
    pub config: VaultConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum VaultConfig {
    Fs { path: PathBuf },
}

impl VaultConfig {
    pub async fn get(&self) -> anyhow::Result<Vault> {
        match &self {
            VaultConfig::Fs { path } => {
                let vault_storage = FileStorage::create(path.clone()).await?;
                let vault = Vault::new(Some(Arc::new(vault_storage)));
                Ok(vault)
            }
        }
    }
}

pub struct IdentitiesState {
    dir: PathBuf,
}

impl IdentitiesState {
    fn new(cli_path: &Path) -> anyhow::Result<Self> {
        let dir = cli_path.join("identities");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn create(&self, name: &str, config: IdentityConfig) -> anyhow::Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        Ok(IdentityState { path, config })
    }

    pub fn get(&self, name: &str) -> anyhow::Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if !path.exists() {
                return Err(anyhow::anyhow!("identity `{name}` does not exist"));
            }
            path
        };
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState { path, config })
    }

    pub fn default(&self) -> anyhow::Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push("default");
            std::fs::canonicalize(&path)?
        };
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState { path, config })
    }

    pub fn set_default(&self, name: &str) -> anyhow::Result<IdentityState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            path
        };
        let link = {
            let mut path = self.dir.clone();
            path.push("default");
            path
        };
        std::os::unix::fs::symlink(&original, &link)?;
        let contents = std::fs::read_to_string(&original)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState {
            path: original,
            config,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentityState {
    path: PathBuf,
    pub config: IdentityConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityConfig {
    vault: VaultConfig,
    identifier: IdentityIdentifier,
    change_history: IdentityChangeHistory,
}

impl IdentityConfig {
    pub async fn new(identity: &Identity<Vault>, vault_config: VaultConfig) -> Self {
        let identifier = identity.identifier().clone();
        let change_history = identity.change_history().await;
        Self {
            vault: vault_config,
            identifier,
            change_history,
        }
    }

    pub async fn get(&self, ctx: &ockam::Context) -> anyhow::Result<Identity<Vault>> {
        let data = self.change_history.export()?;
        let vault = self.vault.get().await?;
        Ok(Identity::import(ctx, &data, &vault).await?)
    }
}

impl PartialEq for IdentityConfig {
    fn eq(&self, other: &Self) -> bool {
        self.vault == other.vault
            && self.identifier == other.identifier
            && self.change_history.compare(&other.change_history)
                == IdentityHistoryComparison::Equal
    }
}

impl Eq for IdentityConfig {}

pub struct NodesState {
    dir: PathBuf,
}

impl NodesState {
    fn new(cli_path: &Path) -> anyhow::Result<Self> {
        let dir = cli_path.join("nodes");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn create(
        &self,
        vault: VaultState,
        identity: IdentityState,
        name: &str,
    ) -> anyhow::Result<NodeState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(name);
            std::fs::create_dir_all(&path)?;
            path
        };
        let state = NodeState::new(path, vault, identity);
        std::fs::write(state.path.join("version"), &state.version)?;
        std::fs::File::create(state.socket())?;
        std::fs::File::create(state.stdout_log())?;
        std::fs::File::create(state.stderr_log())?;
        std::os::unix::fs::symlink(&state.vault.path, state.path.join("vault"))?;
        std::os::unix::fs::symlink(&state.identity.path, state.path.join("identity"))?;
        Ok(state)
    }

    pub fn get(
        &self,
        vault: VaultState,
        identity: IdentityState,
        name: &str,
    ) -> anyhow::Result<NodeState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(name);
            if !path.exists() {
                return Err(anyhow::anyhow!("node `{name}` does not exist"));
            }
            path
        };
        Ok(NodeState::new(path, vault, identity))
    }

    fn vault_name(&self, name: &str) -> anyhow::Result<String> {
        let mut path = self.dir.clone();
        path.push(name);
        path.push("vault");
        let path = std::fs::canonicalize(&path)?;
        file_stem(&path)
    }

    fn identity_name(&self, name: &str) -> anyhow::Result<String> {
        let mut path = self.dir.clone();
        path.push(name);
        path.push("identity");
        let path = std::fs::canonicalize(&path)?;
        file_stem(&path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeState {
    version: String,
    name: String,
    path: PathBuf,
    vault: VaultState,
    identity: IdentityState,
    // authorities: AuthoritiesConfig,
    // setup: NodeSetupConfig, // a mix of the current commands.json with some additional fields to define services
}

impl NodeState {
    fn new(path: PathBuf, vault: VaultState, identity: IdentityState) -> Self {
        Self {
            version: "2.0.0".to_string(),
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path,
            vault,
            identity,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn socket(&self) -> PathBuf {
        self.path.join("socket")
    }

    pub fn stdout_log(&self) -> PathBuf {
        self.path.join("stdout.log")
    }

    pub fn stderr_log(&self) -> PathBuf {
        self.path.join("stderr.log")
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct NodeConfig {
    vault: Option<String>,
    identity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeConfigBuilder {
    vault: Option<String>,
    identity: Option<String>,
}

impl NodeConfigBuilder {
    pub fn new() -> Self {
        Self {
            vault: None,
            identity: None,
        }
    }

    pub fn vault(mut self, name: String) -> Self {
        self.vault = Some(name);
        self
    }

    pub fn identity(mut self, name: String) -> Self {
        self.identity = Some(name);
        self
    }

    pub fn build(self) -> NodeConfig {
        NodeConfig {
            vault: self.vault,
            identity: self.identity,
        }
    }
}

fn file_stem(path: &Path) -> anyhow::Result<String> {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .context("Invalid file name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, Builder};

    #[ockam_macros::test(crate = "ockam")]
    async fn integration(ctx: &mut ockam::Context) -> ockam::Result<()> {
        let rnd_dir = Builder::new().prefix("ockam-").tempdir().unwrap();
        std::env::set_var("OCKAM_HOME", rnd_dir.path());
        let sut = CliState::new().unwrap();

        // Vaults
        let vault_name = {
            let name = hex::encode(&rand::random::<[u8; 4]>());

            let path = rnd_dir.path().join("vaults").join(&format!("{name}.data"));
            let vault_storage = FileStorage::create(path.clone()).await?;
            let vault = Vault::new(Some(Arc::new(vault_storage)));

            let config = VaultConfig::Fs { path };

            let state = sut.vaults.create(&name, config).unwrap();
            let got = sut.vaults.get(&name).unwrap();
            assert_eq!(got, state);

            sut.vaults.set_default(&name).unwrap();
            let got = sut.vaults.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Identities
        let identity_name = {
            let name = hex::encode(&rand::random::<[u8; 4]>());
            let vault_config = sut.vaults.get(&vault_name).unwrap().config;
            let vault = vault_config.get().await.unwrap();
            let identity = Identity::create(ctx, &vault).await.unwrap();
            let identifier =
                IdentityIdentifier::from_key_id(&hex::encode(&rand::random::<[u8; 32]>()));
            let config = IdentityConfig::new(&identity, vault_config).await;

            let state = sut.identities.create(&name, config).unwrap();
            let got = sut.identities.get(&name).unwrap();
            assert_eq!(got, state);

            sut.identities.set_default(&name).unwrap();
            let got = sut.identities.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Nodes
        let node_name = {
            let name = hex::encode(&rand::random::<[u8; 4]>());
            let config = NodeConfig::default();

            let state = sut.create_node(&name, config).unwrap();
            let got = sut.node(&name).unwrap();
            assert_eq!(got, state);

            name
        };

        // Check structure
        let mut expected_entries = vec![
            "vaults".to_string(),
            "vaults/default".to_string(),
            format!("vaults/{vault_name}.json"),
            format!("vaults/{vault_name}.data"),
            "identities".to_string(),
            "identities/default".to_string(),
            format!("identities/{identity_name}.json"),
            "nodes".to_string(),
            format!("nodes/{node_name}"),
        ];
        expected_entries.sort();
        let mut found_entries = vec![];
        sut.dir.read_dir().unwrap().for_each(|entry| {
            let entry = entry.unwrap();
            let dir_name = entry.file_name().into_string().unwrap();
            match dir_name.as_str() {
                "vaults" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_file());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                "identities" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_file());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                "nodes" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_dir());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                _ => panic!("unexpected file"),
            }
        });
        found_entries.sort();
        assert_eq!(expected_entries, found_entries);
        ctx.stop().await?;
        Ok(())
    }
}
