import fluvio_client_py
import asyncio

class AsyncFluvio:
    def __init__(self):
        self._client = fluvio_client_py.AsyncClient()

    async def produce(self, topic, value, key=None):
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, self._client.produce, topic, value, key)

    async def consume(self, topic, partition=0, offset=0):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self._client.consume, topic, partition, offset)

class AsyncFluvioAdmin:
    def __init__(self):
        self._admin = fluvio_client_py.AsyncAdmin()

    async def create_topic(self, name, partitions=1, replication=1):
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, self._admin.create_topic, name, partitions, replication)

    async def delete_topic(self, name):
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, self._admin.delete_topic, name)

    async def list_topics(self):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self._admin.list_topics)
