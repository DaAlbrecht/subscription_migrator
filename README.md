# Subscription Migrator

## Overview

This CLI tool is designed to facilitate the migration of API subscription configurations from XML to YAML format. It provides two main functionalities:

1. **Single Migration**: Convert a single subscription file from XML to YAML.
2. **Bulk Migration**: Search directories for subscription files matching a specified prefix and convert them from XML to YAML.

## Prerequisites

Before using the CLI, ensure that the following environment variables are set:

- `NPR_PLANE_URL`: The control plane URL for non-production environments.
- `PROD_PLANE_URL`: The control plane URL for the production environment.

These environment variables are required for the migration process. If they are not set, the CLI will return an error.

## Installation

### Cargo

To install the `Migrator` CLI using Cargo, run the following command:

```bash
cargo install subscription_migrator
```

Make sure that the Cargo bin directory is in your PATH.

### Binary

To install the `Migrator` CLI using the binary, download the latest release from the [releases page](https://github.com/DaAlbrecht/subscription_migrator/releases)

