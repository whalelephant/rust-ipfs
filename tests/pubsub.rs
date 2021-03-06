use async_std::future::{pending, timeout};
use futures::stream::StreamExt;
use ipfs::{Node, PeerId};
use std::time::Duration;

#[async_std::test]
async fn subscribe_only_once() {
    let a = Node::new().await;
    let _stream = a.pubsub_subscribe("some_topic".into()).await.unwrap();
    a.pubsub_subscribe("some_topic".into()).await.unwrap_err();
}

#[async_std::test]
async fn resubscribe_after_unsubscribe() {
    let a = Node::new().await;

    let mut stream = a.pubsub_subscribe("topic".into()).await.unwrap();
    a.pubsub_unsubscribe("topic").await.unwrap();
    // sender has been dropped
    assert_eq!(stream.next().await, None);

    drop(a.pubsub_subscribe("topic".into()).await.unwrap());
}

#[async_std::test]
async fn unsubscribe_via_drop() {
    let a = Node::new().await;

    let msgs = a.pubsub_subscribe("topic".into()).await.unwrap();
    assert_eq!(a.pubsub_subscribed().await.unwrap(), &["topic"]);

    drop(msgs);

    let empty: &[&str] = &[];
    assert_eq!(a.pubsub_subscribed().await.unwrap(), empty);
}

#[async_std::test]
async fn can_publish_without_subscribing() {
    let a = Node::new().await;
    a.pubsub_publish("topic".into(), b"foobar".to_vec())
        .await
        .unwrap()
}

#[async_std::test]
#[allow(clippy::mutable_key_type)] // clippy doesn't like Vec inside HashSet
async fn publish_between_two_nodes() {
    use futures::stream::StreamExt;
    use std::collections::HashSet;

    let ((a, a_id), (b, b_id)) = two_connected_nodes().await;

    let topic = "shared".to_owned();

    let mut a_msgs = a.pubsub_subscribe(topic.clone()).await.unwrap();
    let mut b_msgs = b.pubsub_subscribe(topic.clone()).await.unwrap();

    // need to wait to see both sides so that the messages will get through
    let mut appeared = false;
    for _ in 0..100usize {
        if a.pubsub_peers(Some(topic.clone()))
            .await
            .unwrap()
            .contains(&b_id)
            && b.pubsub_peers(Some(topic.clone()))
                .await
                .unwrap()
                .contains(&a_id)
        {
            appeared = true;
            break;
        }
        timeout(Duration::from_millis(100), pending::<()>())
            .await
            .unwrap_err();
    }

    assert!(
        appeared,
        "timed out before both nodes appeared as pubsub peers"
    );

    a.pubsub_publish(topic.clone(), b"foobar".to_vec())
        .await
        .unwrap();
    b.pubsub_publish(topic.clone(), b"barfoo".to_vec())
        .await
        .unwrap();

    // the order is not defined, but both should see the other's message and the message they sent
    let expected = [
        (&[topic.clone()], &a_id, b"foobar"),
        (&[topic.clone()], &b_id, b"barfoo"),
    ]
    .iter()
    .cloned()
    .map(|(topics, id, data)| (topics.to_vec(), id.clone(), data.to_vec()))
    .collect::<HashSet<_>>();

    for st in &mut [b_msgs.by_ref(), a_msgs.by_ref()] {
        let actual = st
            .take(2)
            // Arc::try_unwrap will fail sometimes here as the sender side in src/p2p/pubsub.rs:305
            // can still be looping
            .map(|msg| (*msg).clone())
            .map(|msg| (msg.topics, msg.source, msg.data))
            .collect::<HashSet<_>>()
            .await;
        assert_eq!(expected, actual);
    }

    drop(b_msgs);

    let mut disappeared = false;
    for _ in 0..100usize {
        if !a
            .pubsub_peers(Some(topic.clone()))
            .await
            .unwrap()
            .contains(&b_id)
        {
            disappeared = true;
            break;
        }
        timeout(Duration::from_millis(100), pending::<()>())
            .await
            .unwrap_err();
    }

    assert!(disappeared, "timed out before a saw b's unsubscription");
}

async fn two_connected_nodes() -> ((Node, PeerId), (Node, PeerId)) {
    let a = Node::new().await;
    let b = Node::new().await;

    let (a_pk, _) = a.identity().await.unwrap();
    let a_id = a_pk.into_peer_id();

    let (b_pk, mut addrs) = b.identity().await.unwrap();
    let b_id = b_pk.into_peer_id();

    a.connect(addrs.pop().expect("b must have address to connect to"))
        .await
        .unwrap();

    ((a, a_id), (b, b_id))
}
