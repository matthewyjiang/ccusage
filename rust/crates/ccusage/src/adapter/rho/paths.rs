use std::{env, path::PathBuf};

pub(super) const RHO_HOME_ENV: &str = "RHO_HOME";
pub(super) const RHO_DB_FILE_NAME: &str = "usage.sqlite3";

pub(super) fn db_path() -> Option<PathBuf> {
    let root = env::var_os(RHO_HOME_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| crate::home::home_dir().map(|home| home.join(".rho")))?;
    let path = root.join(RHO_DB_FILE_NAME);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use ccusage_test_support::{EnvVarGuard, fs_fixture};

    use super::*;

    #[test]
    fn discovers_database_from_rho_home() {
        let fixture = fs_fixture!({ "usage.sqlite3": "" });
        let _rho_home = EnvVarGuard::set(RHO_HOME_ENV, fixture.root());

        assert_eq!(db_path(), Some(fixture.path(RHO_DB_FILE_NAME)));
    }
}
