FROM aflplusplus/aflplusplus:latest

#build qemu-afl
RUN ln -s $(which afl-showmap) /AFLplusplus && \
    cd /AFLplusplus/qemu_mode && \
    # remove fix
    sed -i 's#test -d qemuafl/.git || { echo#test ! -d qemuafl/.git || { echo#' ./build_qemu_support.sh && \
    ./build_qemu_support.sh

RUN ln -s /AFLplusplus/afl-qemu-trace /usr/local/bin/

RUN apt-get update && apt-get install -qyy python3-pip unzip rsync strace
RUN pip3 install clusterfuzz grpclib protobuf

RUN apt-get install -qyy python2 python-setuptools && \
    cd /opt && \
    git clone https://github.com/jfoote/exploitable.git && \
    cd exploitable && \
    python2 setup.py install

COPY scripts /scripts
RUN chmod +x /scripts/*

COPY docker_pb2.py /scripts/
COPY docker_grpc.py /scripts/

RUN apt-get install -qyy libidn11 libxslt1.1 libssh2-1 libgeoip1 gawk
RUN apt-get clean

CMD ["python", "/scripts/schedule.py"]
