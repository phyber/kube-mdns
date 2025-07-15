// kube-mdns
//
// Update Avahi mDNS with hostnames based on Service labels.
use futures::StreamExt;
use k8s_openapi::api::networking::v1::Ingress;
use kube::{
    Api,
    Client,
};
use kube::runtime::controller::{
    Action,
    Controller,
};
use kube::runtime::watcher;
use std::fmt;
use std::sync::{
    Arc,
};
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{
    debug,
    info,
};
use tracing_subscriber::filter::{
    EnvFilter,
    FromEnvError,
    LevelFilter,
};

mod bus;

// The annotation we look for to take our hostnames from. This should be a
// space separated list.
const HOSTS_ANNOTATION: &str = "phyber.github.io/kube-mdns.hostnames";

// Simple error wrapping for now.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("dbus error")]
    Dbus(#[from] anyhow::Error),

    #[error("kube error")]
    Kube(#[from] kube::Error),

    #[error("tracing subscriber from env error")]
    TracingSubscriberFromEnvError(#[from] FromEnvError),
}

#[derive(Clone)]
struct Context {
    dbus: bus::Dbus,
}

#[derive(Debug)]
struct NamespacedIngress {
    name: String,
}

impl fmt::Display for NamespacedIngress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Ingress should always have a namespace and name, so we should be good to use
// From here and just panic if we don't get one of those.
impl From<&Arc<Ingress>> for NamespacedIngress {
    fn from(ingress: &Arc<Ingress>) -> Self {
        debug!("creating NamespacedIngress from {ingress:?}");

        // Get the name of the ingress. This is used for bookkeeping in our
        // state, since ingress names are used in DNS and must be unique to a
        // namespace. This makes them good for our purposes.
        let name = ingress
            .metadata
            .name
            .as_ref()
            .expect("ingress name");

        // We also grab the namespace to form part of our bookkeeping.
        // There should always(?) at least be namespace set to default.
        let namespace = ingress
            .metadata
            .namespace
            .as_ref()
            .expect("namespace");

        Self {
            name: format!("{namespace}/{name}"),
        }
    }
}

// Attempt to pull the hostnames out of the annotations.
fn ingress_hostnames(ingress: &Arc<Ingress>) -> Option<Vec<String>> {
    debug!("getting mdns hostnames from {ingress:?}");

    let hostnames = ingress
        .metadata
        .annotations
        .clone()?
        .get(HOSTS_ANNOTATION)?
        .to_string();

    // The above could still leave us with an empty string, we can exit here
    // instead of trying to process the hostnames in that case.
    if hostnames.is_empty() {
        return None;
    }

    let hostnames: Vec<String> = hostnames
        .split_whitespace()
        .map(str::to_string)
        .collect();

    Some(hostnames)
}

fn ingress_load_balancer_ips(ingress: &Arc<Ingress>) -> Option<Vec<String>> {
    debug!("getting load balancer IPs from {ingress:?}");

    let ip_addresses: Vec<String> = ingress
        .status
        .clone()?
        .load_balancer?
        .ingress?
        .iter()
        .filter_map(|ingress_load_balancer_ingress| {
            ingress_load_balancer_ingress.ip.clone()
        })
        .collect();

    Some(ip_addresses)
}

fn error_policy(
    _ingress: Arc<Ingress>,
    err: &Error,
    _context: Arc<Mutex<Context>>,
) -> Action {
    info!("error: {err}");

    Action::requeue(Duration::from_secs(5))
}

// The `reconcile` function is called when changes occur, and is responcible
// for actually making the changes that we need.
async fn reconcile(
    ingress: Arc<Ingress>,
    context: Arc<Mutex<Context>>,
) -> Result<Action, Error> {
    let namespaced_ingress = NamespacedIngress::from(&ingress);
    info!("{namespaced_ingress}: reconciling");

    // The resource UID will be our key in the state.
    // We require this for tracking which Ingresses we've configured hostnames
    // for.
    let Some(uid) = &ingress.metadata.uid else {
        info!("{namespaced_ingress}: no UID found, skipping");

        return Ok(Action::await_change());
    };

    let Some(hostnames) = ingress_hostnames(&ingress) else {
        info!("{namespaced_ingress}: no hostnames found, skipping");

        return Ok(Action::await_change());
    };

    info!("{namespaced_ingress}: found annotation set to {hostnames:?}");

    // Get the Ingress IP addresses from the status.
    let Some(ip_addresses) = ingress_load_balancer_ips(&ingress) else {
        info!("{namespaced_ingress}: did not find any load balancer IPs, skipping");

        return Ok(Action::await_change());
    };

    info!("{namespaced_ingress}: found load balancer IP addresses: {ip_addresses:?}");

    // Grab the Dbus client
    let dbus = &mut context
        .lock()
        .await
        .dbus;

    // Unpublish any existing records
    dbus
        .unpublish(uid)
        .await?;

    // If there are no new records to publish, we can stop here
    if hostnames.is_empty() || ip_addresses.is_empty() {
        return Ok(Action::await_change());
    }

    // Publish the new records
    dbus
        .publish(uid, &hostnames, &ip_addresses)
        .await?;

    // Await further changes.
    Ok(Action::await_change())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Configure logging via environment variables. Default to INFO level
    // logging.
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    // Configure Kubernetes API client and Dbus client.
    let client = Client::try_default().await?;
    let ingresses = Api::<Ingress>::all(client.clone());
    let dbus = bus::Dbus::new().await?;

    let context = Arc::new(Mutex::new(Context {
        dbus,
    }));

    Controller::new(ingresses, watcher::Config::default())
        .run(reconcile, error_policy, context)
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}
