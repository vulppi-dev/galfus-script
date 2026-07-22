import { readFileSync } from 'fs';
import { join } from 'path';

// Importa o módulo web gerado
import init, { Playground } from '../../dist/playground-web/galfus_playground.js';

// No target 'web' (usado pelo wasm-bindgen), precisamos inicializar
// passando o buffer do wasm para ser executado no Node/Bun.
const wasmPath = join(import.meta.dir, '../../dist/playground-web/galfus_playground_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);

await init({ module_or_path: wasmBuffer });

const playground = new Playground();

// Capturar saída do buffer de IO em tempo real
playground.setWriteCallback((bytes: Uint8Array) => {
  const text = new TextDecoder().decode(bytes);
  console.log('Interceptado via stdio:', text);
});

playground.setConfig(`
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
`);

// Configura código fonte
playground.setSource(
  'src/main.gfs',
  'export fn main(_args: [[u8]]): i32 { return 0 }',
);

// 1. Checa validade do código
const checkResult = JSON.parse(playground.check());
if (!checkResult.is_valid) {
  console.error('Erros de compilação:', checkResult.diagnostics);
} else {
  console.log('Análise estática (check): OK');
}

// 2. Compila
const compileResult = JSON.parse(playground.compile());
if (!compileResult.ok) {
  console.error('Erro no build:', compileResult.error);
} else {
  console.log('Compilação: OK');
}

// 3. Executa
const runResult = JSON.parse(playground.run(JSON.stringify([])));
if (runResult.error) {
  console.error('Erro no runtime:', runResult.error);
} else {
  console.log('Finalizou com:', runResult.exit_code);
}

// Limpa memória
playground.free();
