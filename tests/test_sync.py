import pytest
from fluvio import Fluvio, FluvioAdmin

def test_topic_produce_and_consume():
    client = Fluvio()
    admin = FluvioAdmin()
    topic = 'test-sync-topic'
    admin.create_topic(topic)
    client.topic_produce(topic, b'hello')
    records = client.topic_consume(topic)
    assert any(b'hello' in r for r in records)
    admin.delete_topic(topic)

def test_admin_list_topics():
    admin = FluvioAdmin()
    topics = admin.list_topics()
    assert isinstance(topics, list)
