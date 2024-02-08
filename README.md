# walletka-server

```
## Example .env file
RUST_LOG=info

# SurrealDB
DB_ENDPOINT=""
DB_USER=""
DB_PASS=""

# Lightning node
LIGHTNING_DATA_DIR="./app_data/ldk_node"
LIGHTNING_NODE_PORT=9876
LIGHTNING_NODE_GRPC_PORT=3000

# LSP
LSP_API_PORT=3002
LSP_CASHU_MINT=lsp
NOSTR_DEFAULT_RELAY=""
DEFAULT_CASHU_ENDPOINT=""

# Cashu
CASHU_MINT_URL=""
CASHU_API_PORT=3001

# RabbitMQ
RABBITMQ_HOST=""
RABBITMQ_PORT=5672
RABBITMQ_USERNAME=""
RABBITMQ_PASSWORD=""
LIGHTNING_NODE_EXCHANGE="walletka.lightning-node"

# Common
ESPLORA_SERVER_URL=""
MNEMONIC="dad erupt orient disease airport produce blade duty angle rail question mutual"
LIGHTNING_NODE_ENDPOINT=""
```
