import pytest
import asyncio
from fluvio import AsyncFluvio, AsyncFluvioAdmin

@pytest.mark.asyncio
def test_async_produce_consume(monkeypatch):
    class DummyAsyncClient:
        def produce(self, topic, value, key=None):
            assert topic == 'test'
            assert value == b'data'
            assert key is None
        def consume(self, topic, partition=0, offset=0):
            assert topic == 'test'
            return [b'data']
    monkeypatch.setattr('fluvio_client_py.AsyncClient', DummyAsyncClient)
    client = AsyncFluvio()
    await client.produce('test', b'data')
    msgs = await client.consume('test')
    assert msgs == [b'data']

@pytest.mark.asyncio
def test_async_admin(monkeypatch):
    class DummyAsyncAdmin:
        def create_topic(self, name, partitions=1, replication=1):
            assert name == 'topic'
            assert partitions == 1
            assert replication == 1
        def delete_topic(self, name):
            assert name == 'topic'
        def list_topics(self):
            return ['topic']
    monkeypatch.setattr('fluvio_client_py.AsyncAdmin', DummyAsyncAdmin)
    admin = AsyncFluvioAdmin()
    await admin.create_topic('topic')
    await admin.delete_topic('topic')
    topics = await admin.list_topics()
    assert topics == ['topic']
