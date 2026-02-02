FROM ubuntu:20.04

RUN apt-get update
RUN apt-get install -y curl unzip
RUN curl -fsS https://raw.githubusercontent.com/fluvio-community/fluvio/master/install.sh | bash

ENV PATH "$PATH:/root/.fluvio/bin"
ENV PATH "$PATH:/root/.fvm/bin"
