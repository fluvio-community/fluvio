import fluvio_client_py

class Fluvio:
    def __init__(self):
        self._client = fluvio_client_py.Client()

    def produce(self, topic, value, key=None):
        self._client.produce(topic, value, key)

    def consume(self, topic, partition=0, offset=0):
        return self._client.consume(topic, partition, offset)

class FluvioAdmin:
    def __init__(self):
        self._admin = fluvio_client_py.Admin()

    def create_topic(self, name, partitions=1, replication=1):
        self._admin.create_topic(name, partitions, replication)

    def delete_topic(self, name):
        self._admin.delete_topic(name)

    def list_topics(self):
        return self._admin.list_topics()
