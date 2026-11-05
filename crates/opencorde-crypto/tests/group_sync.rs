//! Real MLS membership/epoch interoperability, independent providers for every peer.
use opencorde_crypto::provider::PersistentCryptoProvider;
use opencorde_crypto::{encrypt, group, key_package};
use openmls::prelude::*;

fn exercise(size: usize) {
    let providers: Vec<_> = (0..size)
        .map(|_| PersistentCryptoProvider::default())
        .collect();
    let packages: Vec<_> = providers
        .iter()
        .enumerate()
        .map(|(i, p)| key_package::generate_key_package(i as i64 + 1, p).unwrap())
        .collect();
    let credential = CredentialWithKey {
        credential: Credential::new(1i64.to_le_bytes().to_vec(), CredentialType::Basic).unwrap(),
        signature_key: packages[0].1.public().into(),
    };
    let mut groups = vec![group::create_group(credential, &packages[0].1, &providers[0]).unwrap()];
    let mut commits = Vec::new();
    let mut offline = None;
    let mut offline_keys = None;
    for i in 1..size {
        let (commit, welcome) = group::add_member(
            &mut groups[0],
            packages[i].0.clone(),
            &packages[0].1,
            &providers[0],
        )
        .unwrap();
        for j in 1..i {
            let (sender, content) =
                encrypt::decrypt_authenticated(&mut groups[j], &commit, &providers[j]).unwrap();
            assert_eq!(sender, 1);
            assert!(content.is_none());
        }
        groups.push(group::process_welcome(&welcome, &providers[i]).unwrap());
        if i == 1 {
            offline = Some(group::serialize_group(&groups[1]).unwrap());
            offline_keys = Some(providers[1].snapshot().unwrap());
        } else {
            commits.push(commit);
        }
        let expected = (1..=i as i64 + 1).collect::<Vec<_>>();
        for g in &groups {
            assert_eq!(group::member_ids(g).unwrap(), expected);
            assert_eq!(group::epoch(g), i as u64);
        }
    }
    // A restarted offline participant catches up every missed authenticated commit.
    let offline_provider = PersistentCryptoProvider::restore(&offline_keys.unwrap()).unwrap();
    let mut caught = group::deserialize_group(&offline.unwrap()).unwrap();
    for commit in &commits {
        encrypt::decrypt_authenticated(&mut caught, commit, &offline_provider).unwrap();
    }
    assert_eq!(
        group::member_ids(&caught).unwrap(),
        group::member_ids(&groups[0]).unwrap()
    );
    assert_eq!(
        group::export_voice_key(&caught, &offline_provider).unwrap(),
        group::export_voice_key(&groups[0], &providers[0]).unwrap()
    );
    // Every peer sends; all others authenticate identity and decrypt.
    for sender in 0..size {
        let text = format!("hello from {sender} in group {size}");
        let bytes = encrypt::encrypt_message(
            &mut groups[sender],
            text.as_bytes(),
            &packages[sender].1,
            &providers[sender],
        )
        .unwrap();
        for recipient in 0..size {
            if sender != recipient {
                let (identity, plain) = encrypt::decrypt_authenticated(
                    &mut groups[recipient],
                    &bytes,
                    &providers[recipient],
                )
                .unwrap();
                assert_eq!(identity, sender as i64 + 1);
                assert_eq!(plain.unwrap(), text.as_bytes());
            }
        }
    }
    let removed = size - 1;
    let commit =
        group::remove_member(&mut groups[0], size as i64, &packages[0].1, &providers[0]).unwrap();
    for i in 1..removed {
        encrypt::decrypt_authenticated(&mut groups[i], &commit, &providers[i]).unwrap();
    }
    let ciphertext = encrypt::encrypt_message(
        &mut groups[0],
        b"after removal",
        &packages[0].1,
        &providers[0],
    )
    .unwrap();
    assert!(
        encrypt::decrypt_message(&mut groups[removed], &ciphertext, &providers[removed]).is_err()
    );
    for i in 1..removed {
        assert_eq!(
            encrypt::decrypt_message(&mut groups[i], &ciphertext, &providers[i])
                .unwrap()
                .unwrap(),
            b"after removal"
        );
    }
    // A commit from another group cannot be accepted.
    let foreign = group::create_group(
        CredentialWithKey {
            credential: Credential::new(1i64.to_le_bytes().to_vec(), CredentialType::Basic)
                .unwrap(),
            signature_key: packages[0].1.public().into(),
        },
        &packages[0].1,
        &providers[0],
    )
    .unwrap();
    let mut foreign = foreign;
    assert!(encrypt::decrypt_authenticated(&mut foreign, &commit, &providers[0]).is_err());
}
#[test]
fn five_members_offline_add_remove() {
    exercise(5);
}
#[test]
fn twenty_members_offline_add_remove() {
    exercise(20);
}
