use async_std::task;

#[test]
fn multiple_consecutive_ephemeral_listening_addresses() {
    task::block_on(async move {
        let node = ipfs::Node::new().await;

        let target = libp2p::build_multiaddr!(Ip4([127, 0, 0, 1]), Tcp(0u16));

        let first = node.add_listening_address(target.clone()).await.unwrap();
        assert_ne!(target, first);

        let second = node.add_listening_address(target.clone()).await.unwrap();
        assert_ne!(target, second);
        assert_ne!(first, second);
    });
}

#[test]
fn multiple_concurrent_ephemeral_listening_addresses_on_same_ip() {
    task::block_on(async move {
        let node = ipfs::Node::new().await;

        let target = libp2p::build_multiaddr!(Ip4([127, 0, 0, 1]), Tcp(0u16));

        let first = node.add_listening_address(target.clone());
        let second = node.add_listening_address(target);

        let (first, second) = futures::future::join(first, second).await;

        // before we have an Swarm-alike api on the background task to make sure the two futures
        // (first and second) would attempt to modify the background task before a poll to the
        // inner swarm, this will produce one or two successes.
        //
        // with two attempts without polling the swarm in the between:
        // assert_eq!(first.is_ok(), second.is_err());
        //
        // intuitively it could seem that first will always succeed because it must get the first
        // attempt to push messages into the queue but not sure if that should be leaned on.

        assert!(
            first.is_ok() || second.is_ok(),
            "first: {:?}, second: {:?}",
            first,
            second
        );
    });
}

#[test]
#[cfg(not(target_os = "macos"))]
fn multiple_concurrent_ephemeral_listening_addresses_on_different_ip() {
    task::block_on(async move {
        let node = ipfs::Node::new().await;

        // it doesnt work on mac os x as 127.0.0.2 is not enabled by default.
        let first =
            node.add_listening_address(libp2p::build_multiaddr!(Ip4([127, 0, 0, 1]), Tcp(0u16)));
        let second =
            node.add_listening_address(libp2p::build_multiaddr!(Ip4([127, 0, 0, 2]), Tcp(0u16)));

        let (first, second) = futures::future::join(first, second).await;

        // both should succeed
        first.unwrap();
        second.unwrap();
    });
}

#[test]
fn adding_unspecified_addr_resolves_with_first() {
    task::block_on(async move {
        let node = ipfs::Node::new().await;
        // there is no test in trying to match this with others as ... that would be quite
        // perilous.
        node.add_listening_address(libp2p::build_multiaddr!(Ip4([0, 0, 0, 0]), Tcp(0u16)))
            .await
            .unwrap();
    });
}

#[test]
fn listening_for_multiple_unspecified_addresses() {
    task::block_on(async move {
        let node = ipfs::Node::new().await;
        // there is no test in trying to match this with others as ... that would be quite
        // perilous.
        let target = libp2p::build_multiaddr!(Ip4([0, 0, 0, 0]), Tcp(0u16));
        let first = node.add_listening_address(target.clone());
        let second = node.add_listening_address(target);

        let (first, second) = futures::future::join(first, second).await;

        // this test is a bad one similar to multiple_concurrent_ephemeral_listening_addresses_on_same_ip
        // see also https://github.com/rs-ipfs/rust-ipfs/issues/194 for more discussion.

        // the other should be denied because there is a pending incomplete when trying to listen
        // on unspecified address
        assert!(
            first.is_ok() || second.is_ok(),
            "first: {:?}, second: {:?}",
            first,
            second
        );
    });
}

#[test]
fn remove_listening_address() {
    task::block_on(async move {
        let node = ipfs::Node::new().await;

        let unbound = libp2p::build_multiaddr!(Ip4([127, 0, 0, 1]), Tcp(0u16));
        let first = node.add_listening_address(unbound.clone()).await.unwrap();

        // the book keeping changes from matching the unbound address to the bound one returned
        // from the future.
        node.remove_listening_address(unbound.clone())
            .await
            .unwrap_err();
        node.remove_listening_address(first).await.unwrap();
    });
}

#[test]
#[ignore]
fn remove_listening_address_before_completing() {
    // TODO: cannot test this before we have a way of getting between the queue used to communicate
    // with the IpfsFuture (or better yet, construct one ourselves here in the test) to make sure
    // we can push a IpfsEvent::AddListenerAddress followed by an IpfsEvent::RemoveListenerAddress
    // "immediatedly".
}
