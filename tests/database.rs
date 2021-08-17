use utils::{tmpdir};
use leveldb_mcpe::database::{Database};
use leveldb_mcpe::options::{Options};

#[test]
fn test_create_options() {
  Options::new();
}

#[test]
fn test_open_database() {
  let mut opts = Options::new();
  opts.create_if_missing = true;
  let tmp = tmpdir("create_if_missing");
  let res: Result<Database<i32>,_> = Database::open(tmp.path(), opts);
  assert!(res.is_ok());
}

#[test]
fn test_open_non_existant_database_without_create() {
  let mut opts = Options::new();
  opts.create_if_missing = false;
  let tmp = tmpdir("missing");
  let res: Result<Database<i32>,_> = Database::open(tmp.path(), opts);
  assert!(res.is_err());
}
