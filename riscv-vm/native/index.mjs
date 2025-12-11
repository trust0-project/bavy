/**
 * ESM wrapper for native addon.
 */
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const native = require('./index.js');

export const { ConnectionStatus, WebTransportClient } = native;





