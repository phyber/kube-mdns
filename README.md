# kube-mdns

Kube mDNS registers hostnames with Avahi based on annotations given to an
`Ingress`. This allows Ingresses to have unique `.local` hostnames assigned to
them, which helps with reverse proxying.

This project is based on [`docker-mdns`], which provides the same functionality
for Docker environments.

## Minimum Supported Rust Version (MSRV)

The MSRV for this project is currently 1.85.1.

## Configuration

Kube mDNS does not take any command line arguments, but will take logging
configuration from the environment via the `RUST_LOG` environment variable.
Examples for setting this can be found in the `tracing_subscriber`
[`EnvFilter`] documentation.

The main Kube mDNS configuration is done through annotations on `Ingress`
objects in Kubernetes.

| Label                                  | Description                         |
|----------------------------------------|-------------------------------------|
| `phyber.github.io/kube-mdns.hostnames` | Hostnames to add for this `Ingress` |

The `phyber.github.io/kube-mdns.hostnames` annotation can take a list of
whitespace separated hostnames if you want multiple hostnames for a single
`Ingress`.

The IP addresses that the hostnames point to will be taken from the IP address
of the `LoadBalancer` that ends up attached to the `Ingress` object.

## Example

```yaml
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  annotations:
    nginx.ingress.kubernetes.io/backend-protocol: "HTTP"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    phyber.github.io/kube-mdns.hostnames: "example.local"
spec:
  ingressClassName: "nginx"
  rules:
    - host: "example.local"
      http:
        paths:
          - path: "/"
            pathType: "ImplementationSpecific"
            backend:
              service:
                name: "example"
                port:
                  name: "example"
```

## License

Licensed under either of

  * Apache License, Version 2.0
    ([LICENSE-APACHE] or https://www.apache.org/licenses/LICENSE-2.0)
  * MIT license
    ([LICENSE-MIT] or https://opensource.org/licenses/MIT)

at your option.

<!-- links -->
[`docker-mdns`]: https://github.com/phyber/docker-mdns
[`EnvFilter`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
[LICENSE-APACHE]: LICENSE-APACHE
[LICENSE-MIT]: LICENSE-MIT
