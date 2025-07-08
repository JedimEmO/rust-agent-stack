import { spawn } from 'child_process';
import { existsSync } from 'fs';
import { join, resolve } from 'path';
import type { Plugin } from 'vite';

interface WasmPackOptions {
  cratePath: string;
  outDir?: string;
  outName?: string;
  features?: string[];
  watchMode?: boolean;
}

export function wasmPack(options: WasmPackOptions): Plugin {
  const {
    cratePath,
    outDir = 'public/pkg',
    outName,
    features = ['wasm-client'],
    watchMode = true,
  } = options;

  const absoluteCratePath = resolve(cratePath);
  const absoluteOutDir = resolve(outDir);
  
  let isBuilding = false;
  let buildPromise: Promise<void> | null = null;

  const buildWasm = async () => {
    if (isBuilding) {
      return buildPromise;
    }

    isBuilding = true;
    console.log(`🦀 Building WASM package from ${cratePath}...`);

    buildPromise = new Promise<void>((resolve, reject) => {
      const args = [
        'build',
        '--target', 'web',
        '--out-dir', absoluteOutDir,
        '--no-default-features',
      ];

      if (features.length > 0) {
        args.push('--features', features.join(','));
      }

      if (outName) {
        args.push('--out-name', outName);
      }

      const wasmPack = spawn('wasm-pack', args, {
        cwd: absoluteCratePath,
        stdio: 'inherit',
      });

      wasmPack.on('close', (code) => {
        isBuilding = false;
        buildPromise = null;
        
        if (code === 0) {
          console.log('✅ WASM package built successfully!');
          resolve();
        } else {
          reject(new Error(`wasm-pack exited with code ${code}`));
        }
      });

      wasmPack.on('error', (err) => {
        isBuilding = false;
        buildPromise = null;
        
        if (err.message.includes('ENOENT')) {
          console.error('❌ wasm-pack not found. Please install it: https://rustwasm.github.io/wasm-pack/installer/');
        }
        reject(err);
      });
    });

    return buildPromise;
  };

  return {
    name: 'vite-plugin-wasm-pack',
    
    async buildStart() {
      // Check if wasm-pack is installed
      try {
        await new Promise((resolve, reject) => {
          const checkWasmPack = spawn('wasm-pack', ['--version'], { stdio: 'pipe' });
          checkWasmPack.on('close', (code) => {
            if (code === 0) resolve(undefined);
            else reject(new Error('wasm-pack not found'));
          });
          checkWasmPack.on('error', reject);
        });
      } catch {
        throw new Error('wasm-pack is not installed. Please install it from: https://rustwasm.github.io/wasm-pack/installer/');
      }

      // Build on start
      await buildWasm();
    },

    configureServer(server) {
      if (!watchMode) return;

      // Watch Rust source files for changes
      const srcPath = join(absoluteCratePath, 'src');
      const cargoToml = join(absoluteCratePath, 'Cargo.toml');
      
      server.watcher.add([srcPath, cargoToml]);
      
      server.watcher.on('change', async (path) => {
        if (path.startsWith(srcPath) || path === cargoToml) {
          console.log(`🔄 Rust source changed, rebuilding WASM...`);
          try {
            await buildWasm();
            // Trigger HMR
            server.ws.send({ type: 'full-reload' });
          } catch (err) {
            console.error('❌ WASM build failed:', err);
          }
        }
      });
    },
  };
}