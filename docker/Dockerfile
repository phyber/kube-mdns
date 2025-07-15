# vim:ft=dockerfile:
ARG BASE_IMAGE
ARG IMAGE_ARCH
FROM --platform=linux/${IMAGE_ARCH} ${BASE_IMAGE}

ARG TARGET
COPY target/${TARGET}/release/kube-mdns /usr/local/bin/

ENTRYPOINT ["kube-mdns"]
