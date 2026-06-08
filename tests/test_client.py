import pytest
from fluvio import Fluvio, FluvioAdmin

def test_produce_consume(monkeypatch):
    class DummyClient:
        def produce(self, topic, value, key=None):
            assert topic == 'test'
            assert value == b'data'
            assert key is None
        def consume(self, topic, partition=0, offset=0):
            assert topic == 'test'
            return [b'data']
    monkeypatch.setattr('fluvio_client_py.Client', DummyClient)
    client = Fluvio()
    client.produce('test', b'data')
    msgs = client.consume('test')
    assert msgs == [b'data']

def test_admin(monkeypatch):
    class DummyAdmin:
        def create_topic(self, name, partitions=1, replication=1):
            assert name == 'topic'
            assert partitions == 1
            assert replication == 1
        def delete_topic(self, name):
            assert name == 'topic'
        def list_topics(self):
            return ['topic']
    monkeypatch.setattr('fluvio_client_py.Admin', DummyAdmin)
    admin = FluvioAdmin()
    admin.create_topic('topic')
    admin.delete_topic('topic')
    topics = admin.list_topics()
    assert topics == ['topic']
