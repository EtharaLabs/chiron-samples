import os
from prometheus_client import multiprocess

wsgi_app = 'argus_backend:create_app()'
worker_class = 'uvicorn.workers.UvicornWorker'
workers = 4
bind = 'unix:/var/lib/argus/argus.sock'
# The application accepts Nginx's peer address only over the private Unix socket.
forwarded_allow_ips = ''
umask = 0o007
backlog = 1024
errorlog = '/var/log/argus/argus.log'
max_requests = 65535
graceful_timeout = 60


def child_exit(server, worker):
    if os.environ.get('PROMETHEUS_MULTIPROC_DIR'):
        multiprocess.mark_process_dead(worker.pid)
