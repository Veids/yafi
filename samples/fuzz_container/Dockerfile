FROM aflplusplus/aflplusplus:latest

#build qemu-afl
RUN ln -s $(which afl-showmap) /AFLplusplus && \
    cd /AFLplusplus/qemu_mode && \
    # remove fix
    sed -i 's#test -d qemuafl/.git || { echo#test ! -d qemuafl/.git || { echo#' ./build_qemu_support.sh && \
    ./build_qemu_support.sh

RUN ln -s /AFLplusplus/afl-qemu-trace /usr/local/bin/

RUN apt-get update && apt-get install -qyy libidn11 libxslt1.1 libssh2-1 libgeoip1 gawk strace unzip rsync
RUN apt-get clean

RUN mkdir /scripts
COPY ./schedule.sh /scripts
COPY ./schedule.py /scripts

RUN chmod +x /scripts/*
CMD ["python", "/scripts/schedule.py"]
