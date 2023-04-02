FROM alpine:latest

# Install necessary packages
RUN apk add --no-cache \
    busybox-extras

# Expose the ports you want to open
EXPOSE 8000 8001 8002

# Run a simple TCP server on each port
CMD ["sh", "-c", "tcpsvd -vE 0.0.0.0 8000 echo | tcpsvd -vE 0.0.0.0 8001 echo | tcpsvd -vE 0.0.0.0 8002 echo"]

