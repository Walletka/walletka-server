use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{Channel, ExchangeDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
};
use anyhow::Result;

pub async fn get_rabbitmq_connection(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
) -> Result<Connection> {
    // open a connection to RabbitMQ server
    let connection = Connection::open(&OpenConnectionArguments::new(
        host, port, username, password,
    ))
    .await
    .unwrap();

    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();

    Ok(connection)
}

pub async fn ensure_exchange_created(
    channel: &Channel,
    exhange_name: &str,
    exchange_type: &str,
) -> Result<()> {
    channel.register_callback(DefaultChannelCallback).await?;

    channel
        .exchange_declare(ExchangeDeclareArguments::new(exhange_name, exchange_type))
        .await?;

    Ok(())
}
