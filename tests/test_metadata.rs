//! Test metadata fetch, group membership, consumer metadata.
extern crate env_logger;
extern crate futures;
extern crate rand;
extern crate rdkafka;

use futures::*;

use rdkafka::consumer::Consumer;
use rdkafka::topic_partition_list::TopicPartitionList;

mod utils;
use utils::*;


#[test]
fn test_metadata() {
    let _r = env_logger::init();

    let topic_name = rand_test_topic();
    produce_messages(&topic_name, 1, &value_fn, &key_fn, Some(0), None);
    produce_messages(&topic_name, 1, &value_fn, &key_fn, Some(1), None);
    produce_messages(&topic_name, 1, &value_fn, &key_fn, Some(2), None);
    let consumer = create_stream_consumer(&rand_test_group(), None);

    let metadata = consumer.fetch_metadata(None, 5000).unwrap();

    let topic_metadata = metadata.topics().iter()
        .find(|m| m.name() == topic_name).unwrap();

    let mut ids = topic_metadata.partitions().iter().map(|p| p.id()).collect::<Vec<_>>();
    ids.sort();

    assert_eq!(ids, vec![0, 1, 2]);
    // assert_eq!(topic_metadata.error(), None);
    assert_eq!(topic_metadata.partitions().len(), 3);
    assert_eq!(topic_metadata.partitions()[0].leader(), 0);
    assert_eq!(topic_metadata.partitions()[1].leader(), 0);
    assert_eq!(topic_metadata.partitions()[2].leader(), 0);
    assert_eq!(topic_metadata.partitions()[0].replicas(), &[0]);
    assert_eq!(topic_metadata.partitions()[0].isr(), &[0]);

    let metadata_one_topic = consumer.fetch_metadata(Some(&topic_name), 5000).unwrap();
    assert_eq!(metadata_one_topic.topics().len(), 1);
}

#[test]
fn test_subscription() {
    let _r = env_logger::init();

    let topic_name = rand_test_topic();
    produce_messages(&topic_name, 10, &value_fn, &key_fn, None, None);
    let consumer = create_stream_consumer(&rand_test_group(), None);
    consumer.subscribe(&[topic_name.as_str()]).unwrap();

    let _consumer_future = consumer.start().take(10).wait();

    let mut tpl = TopicPartitionList::new();
    tpl.add_topic_unassigned(&topic_name);
    assert_eq!(tpl, consumer.subscription().unwrap());
}

#[test]
fn test_group_membership() {
    let _r = env_logger::init();

    let topic_name = rand_test_topic();
    let group_name = rand_test_group();
    produce_messages(&topic_name, 1, &value_fn, &key_fn, Some(0), None);
    produce_messages(&topic_name, 1, &value_fn, &key_fn, Some(1), None);
    produce_messages(&topic_name, 1, &value_fn, &key_fn, Some(2), None);
    let consumer = create_stream_consumer(&group_name, None);
    consumer.subscribe(&[topic_name.as_str()]).unwrap();

    // Make sure the consumer joins the group
    let _consumer_future = consumer.start()
        .take(1)
        .for_each(|_| Ok(()))
        .wait();

    let group_list = consumer.fetch_group_list(None, 5000).unwrap();

    // Print all the data, valgrind will check memory access
    for group in group_list.groups().iter() {
        println!("{} {} {} {}", group.name(), group.state(), group.protocol(), group.protocol_type());
        for member in group.members() {
            println!("  {} {} {}", member.id(), member.client_id(), member.client_host());
        }
    }

    let group_list2 = consumer.fetch_group_list(Some(&group_name), 5000).unwrap();
    assert_eq!(group_list2.groups().len(), 1);

    let consumer_group = group_list2.groups().iter().find(|&g| g.name() == group_name).unwrap();
    assert_eq!(consumer_group.members().len(), 1);

    let consumer_member = &consumer_group.members()[0];
    assert_eq!(consumer_member.client_id(), "rdkafka_integration_test_client");
}
