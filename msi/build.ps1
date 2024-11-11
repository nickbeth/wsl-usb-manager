# Copyright (c) 2024 JP Hutchins
# SPDX-License-Identifier: MIT

# Build the release
cargo build --release

# Clean the msi directory
dotnet clean msi

# mkdir the staging directory if it doesn't exist
New-Item -ItemType Directory -Path msi/staging -Force

# Copy the files to the staging directory
Copy-Item target/release/wsl-usb-manager.exe msi/staging/wsl-usb-manager.exe
Copy-Item LICENSE.md msi/staging/LICENSE.md

# Run a dotnet build with some environment variables set
$env:COMPANY_NAME = "WSL USB Manager"
$env:PRODUCT_NAME = "WSL USB Manager"
$env:APP_NAME = "wsl-usb-manager"
$env:EXE_NAME = "wsl-usb-manager.exe"
$env:VERSION = (Get-Content Cargo.toml | Select-String -Pattern '^version\s*=\s*"(.*)"' | ForEach-Object { $_.Matches[0].Groups[1].Value })
$env:PORTABLE_PATH = "staging"
$env:MSI_NAME = "WSL USB Manager"

dotnet build msi --configuration Release
