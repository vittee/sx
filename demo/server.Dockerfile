# A plain Debian sshd with bash, vim and tmux: the "server" the demo logs in to.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends openssh-server vim tmux less \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /run/sshd \
    && ssh-keygen -A \
    && : > /etc/motd \
    && printf 'PrintMotd no\nPrintLastLog no\n' >> /etc/ssh/sshd_config \
    && useradd -m -s /bin/bash demo \
    && usermod -p '*' demo \
    && mkdir -p /home/demo/.ssh

COPY demo/.keys/id_ed25519.pub /home/demo/.ssh/authorized_keys
COPY demo/server.bashrc /home/demo/.bashrc
RUN chown -R demo:demo /home/demo && chmod 700 /home/demo/.ssh && chmod 600 /home/demo/.ssh/authorized_keys

EXPOSE 22
CMD ["/usr/sbin/sshd", "-D", "-e"]
