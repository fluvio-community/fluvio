import pytest
import asyncio
from fluvio import AsyncFluvio, AsyncFluvioAdmin

@pytest.mark.asyncio
async def test_async_topic_produce_and_consume():
    client = AsyncFluvio()
    admin = AsyncFluvioAdmin()
    topic = 'test-async-topic'
    await admin.create_topic(topic)
    await client.topic_produce(topic, b'async-hello')
    records = await client.topic_consume(topic)
    assert any(b'async-hello' in r for r in records)
    await admin.delete_topic(topic)

@pytest.mark.asyncio
async def test_async_admin_list_topics():
    admin = AsyncFluvioAdmin()
    topics = await admin.list_topics()
    assert isinstance(topics, list)
