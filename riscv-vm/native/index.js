/* tslint:disable */
/* eslint-disable */
/* prettier-ignore */

/**
 * Native bindings loader for riscv-vm
 * 
 * This module detects the current platform and loads the appropriate
 * pre-built native binary. All binaries are bundled in this package.
 * 
 * Supported platforms:
 * - darwin-x64 (macOS Intel)
 * - darwin-arm64 (macOS Apple Silicon)
 * - linux-x64-gnu (Linux x64 glibc)
 * - linux-x64-musl (Linux x64 musl/Alpine)
 * - linux-arm64-gnu (Linux ARM64 glibc)
 * - linux-arm64-musl (Linux ARM64 musl/Alpine)
 * - win32-x64-msvc (Windows x64)
 */

const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let loadError = null

/**
 * Detect if running on musl libc (Alpine Linux, etc.)
 */
function isMusl() {
  if (!process.report || typeof process.report.getReport !== 'function') {
    try {
      const lddPath = require('child_process').execSync('which ldd').toString().trim()
      return readFileSync(lddPath, 'utf8').includes('musl')
    } catch (e) {
      return true
    }
  } else {
    const { glibcVersionRuntime } = process.report.getReport().header
    return !glibcVersionRuntime
  }
}

/**
 * Load a native binding from the native directory
 */
function loadNativeBinding(filename) {
  const filepath = join(__dirname, filename)
  if (!existsSync(filepath)) {
    throw new Error(
      `Native binding not found: ${filename}\n` +
      `Expected at: ${filepath}\n` +
      `Platform: ${platform}, Architecture: ${arch}\n\n` +
      `This platform may not be supported, or the package was not installed correctly.\n` +
      `Try reinstalling the package: npm install virtual-machine`
    )
  }
  return require(filepath)
}

/**
 * Determine the correct native binding filename for the current platform
 */
function getNativeBindingFilename() {
  switch (platform) {
    case 'darwin':
      switch (arch) {
        case 'x64':
          return 'riscv-vm-native.darwin-x64.node'
        case 'arm64':
          return 'riscv-vm-native.darwin-arm64.node'
        default:
          throw new Error(`Unsupported architecture on macOS: ${arch}`)
      }

    case 'linux':
      const musl = isMusl()
      switch (arch) {
        case 'x64':
          return musl 
            ? 'riscv-vm-native.linux-x64-musl.node'
            : 'riscv-vm-native.linux-x64-gnu.node'
        case 'arm64':
          return musl
            ? 'riscv-vm-native.linux-arm64-musl.node'
            : 'riscv-vm-native.linux-arm64-gnu.node'
        default:
          throw new Error(`Unsupported architecture on Linux: ${arch}`)
      }

    case 'win32':
      switch (arch) {
        case 'x64':
          return 'riscv-vm-native.win32-x64-msvc.node'
        default:
          throw new Error(`Unsupported architecture on Windows: ${arch}`)
      }

    default:
      throw new Error(`Unsupported platform: ${platform}`)
  }
}

// Load the native binding
try {
  const filename = getNativeBindingFilename()
  nativeBinding = loadNativeBinding(filename)
} catch (e) {
  loadError = e
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error('Failed to load native binding')
}

// Export the native binding APIs
const { ConnectionStatus, WebTransportClient } = nativeBinding

module.exports.ConnectionStatus = ConnectionStatus
module.exports.WebTransportClient = WebTransportClient
