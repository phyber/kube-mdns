// DBus handling.
use anyhow::Result;
use std::collections::HashMap;
use tracing::{
    debug,
    info,
};
use zbus::{
    fdo,
    proxy,
    Connection,
};
use zbus::zvariant::OwnedObjectPath;

// Name our magic numbers similar to the Avahi source code.
// These are taken from Avahi docs at https://avahi.org/doxygen/html/
const AVAHI_IF_UNSPEC: i32 = -1;
const AVAHI_PROTO_UNSPEC: i32 = -1;
const FLAG_NO_REVERSE: u32 = 16;

#[proxy(
    default_path = "/",
    default_service = "org.freedesktop.Avahi",
    interface = "org.freedesktop.Avahi.Server",
)]
trait AvahiServer {
    // EntryGroupNew
    fn entry_group_new(&self) -> fdo::Result<OwnedObjectPath>;

    // GetNetworkInterfaceIndexByName
    fn get_network_interface_index_by_name(
        &self,
        interface_name: &str,
    ) -> fdo::Result<i32>;
}

#[proxy(
    default_service = "org.freedesktop.Avahi",
    interface = "org.freedesktop.Avahi.EntryGroup",
)]
trait EntryGroup {
    // AddAddress
    fn add_address(
        &self,
        index: &i32,
        protocol: i32,
        flags: u32,
        host: &str,
        address: String,
    ) -> fdo::Result<()>;

    // Commit
    fn commit(&self) -> fdo::Result<()>;

    // Free
    fn free(&self) -> fdo::Result<()>;

    // Reset
    fn reset(&self) -> fdo::Result<()>;
}

#[derive(Clone, Debug)]
pub struct Dbus {
    conn: Connection,
    published: HashMap<String, OwnedObjectPath>,
}

impl Dbus {
    pub async fn new() -> Result<Self> {
        info!("Getting D-Bus handle");

        let dbus = Self {
            conn: Connection::system().await?,
            published: HashMap::new(),
        };

        Ok(dbus)
    }

    pub async fn publish(
        &mut self,
        uid: &str,
        hosts: &[String],
        ip_addresses: &[String],
    ) -> Result<()> {
        info!("Publishing config: {uid:?}");

        if hosts.is_empty() || ip_addresses.is_empty() {
            return Ok(())
        }

        // Get a new group to publish under
        let proxy = AvahiServerProxy::new(&self.conn).await?;
        let group_path = proxy.entry_group_new().await?;

        let entry_group = EntryGroupProxy::builder(&self.conn)
            .path(&group_path)?
            .build()
            .await?;

        for address in ip_addresses {
            debug!("AddAddress: {address:?}");

            for host in hosts {
                entry_group.add_address(
                    &AVAHI_IF_UNSPEC,
                    AVAHI_PROTO_UNSPEC,
                    FLAG_NO_REVERSE,
                    host,
                    address.to_string(),
                ).await?;
            }
        }

        entry_group.commit().await?;

        debug!("Addresses committed");

        self.published.insert(uid.to_string(), group_path);

        Ok(())
    }

    pub async fn unpublish(
        &mut self,
        uid: &str,
    ) -> Result<()> {
        info!("Unpublishing config: {uid:?}");

        let Some(group_path) = self.published.remove(uid) else {
            return Ok(())
        };

        let entry_group = EntryGroupProxy::builder(&self.conn)
            .path(&group_path)?
            .build()
            .await?;

        entry_group.reset().await?;
        entry_group.free().await?;

        debug!("Unpublished: {uid}");

        Ok(())
    }
}
