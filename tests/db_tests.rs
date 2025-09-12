use file_organizer::{index::Db, scanner::RawFileMetadata};
use tokio::task;
use std::{path::{Path, PathBuf}, time::SystemTime};

#[tokio::test]
async fn test_update_and_lookup_file() {
    let db = Db::new(Path::new(":memory:")).await.unwrap();

    let temp_path = Path::new("testfile.txt");
    tokio::fs::write(&temp_path, "hello").await.unwrap();

    let meta = RawFileMetadata {
        path: temp_path.to_path_buf(),
        size: 5,
        created: Some(SystemTime::now()),
        modified: Some(SystemTime::now()),
        accessed: Some(SystemTime::now()),
        permissions: tokio::fs::metadata(&temp_path).await.unwrap().permissions(),
        is_file: true,
        is_dir: false,
        is_symlink: false,
    };

    db.update_file(&meta, "text", Path::new("dest/testfile.txt"), "hash123")
        .await
        .unwrap();

    let looked_up = db.lookup(temp_path).await.unwrap();
    assert!(looked_up.is_some());
    assert_eq!(looked_up.unwrap().size, 5);

    tokio::fs::remove_file(&temp_path).await.unwrap();
}


#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_writes_are_safe() {
    let db = Db::new(Path::new(":memory:")).await.unwrap();

    // create temp files
    for i in 0..5 {
        let path = format!("tempfile_{i}.txt");
        tokio::fs::write(&path, format!("data{i}")).await.unwrap();
    }

    // spawn 5 concurrent update_file tasks
    let mut handles = vec![];
    for i in 0..5 {
        let db = db.clone();
        let path = Path::new(&format!("tempfile_{i}.txt")).to_path_buf();
        handles.push(task::spawn(async move {
            let meta = RawFileMetadata {
                path: path.clone(),
                size: 5,
                created: Some(SystemTime::now()),
                modified: Some(SystemTime::now()),
                accessed: Some(SystemTime::now()),
                permissions: tokio::fs::metadata(&path).await.unwrap().permissions(),
                is_file: true,
                is_dir: false,
                is_symlink: false,
            };

            db.update_file(&meta, "text", Path::new(&format!("dest_{i}.txt")), &format!("hash{i}"))
                .await
                .unwrap();
        }));
    }

    // wait for all tasks
    for h in handles {
        h.await.unwrap();
    }

    // verify all files are present in DB
    for i in 0..5 {
        let path = PathBuf::from(format!("tempfile_{i}.txt"));
        let entry = db.lookup_full(&path).await.unwrap();
        assert!(entry.is_some(), "File {i} missing in DB");

        let e = entry.unwrap();
        assert_eq!(e.hash.unwrap(), format!("hash{i}"));
        assert_eq!(e.dest_path.to_string_lossy(), format!("dest_{i}.txt"));
    }

    // cleanup
    for i in 0..5 {
        let path = format!("tempfile_{i}.txt");
        tokio::fs::remove_file(&path).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_updates_on_same_file() {
    let db = Db::new(Path::new(":memory:")).await.unwrap();

    // temp file to simulate concurrent writes
    let path = Path::new("stress_test.txt");
    tokio::fs::write(&path, "initial").await.unwrap();

    let mut handles = vec![];

    for i in 0..100 {
        let db = db.clone();
        let path = path.to_path_buf();

        handles.push(task::spawn(async move {
            let meta = RawFileMetadata {
                path: path.clone(),
                size: 7,
                created: Some(SystemTime::now()),
                modified: Some(SystemTime::now()),
                accessed: Some(SystemTime::now()),
                permissions: tokio::fs::metadata(&path).await.unwrap().permissions(),
                is_file: true,
                is_dir: false,
                is_symlink: false,
            };

            let dest = Path::new(&format!("dest_{i}.bin")).to_path_buf();
            let hash = format!("hash{i}");

            db.update_file(&meta, "bin", &dest, &hash)
                .await
                .unwrap();

            i // return index for debugging
        }));
    }

    // wait for all tasks
    for h in handles {
        h.await.unwrap();
    }

    // lookup after all updates
    let entry = db.lookup_full(path).await.unwrap().unwrap();

    println!("Final entry after stress test: {:?}", entry);

    // only last update should persist due to ON CONFLICT DO UPDATE
    assert!(entry.hash.unwrap().starts_with("hash"));
    assert!(entry.dest_path.to_string_lossy().starts_with("dest_"));

    // cleanup
    tokio::fs::remove_file(path).await.unwrap();
}

