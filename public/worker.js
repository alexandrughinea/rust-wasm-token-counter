import init, { count_tokens_from_raw_text, set_token_regex, init_panic_hook, get_file_preview, count_tokens_from_file } from './pkg/rust_wasm_token_counter.js';

init()
    .then(() => {
        init_panic_hook();
        self.postMessage({type: 'init_complete'});
    })
    .catch(err => {
        self.postMessage({
            type: 'error',
            message: 'Failed to initialize WASM: ' + err.toString()
        });
    });

async function processInput(input) {
    const PREVIEW_SIZE = 500;
    const RAW_TEXT_LIMIT = 100000
    try {
        let result;
        if (input instanceof File) {
            const preview = await get_file_preview(input, PREVIEW_SIZE);
            self.postMessage({
                type: 'preview',
                content: preview
            });

            result = await count_tokens_from_file(input, (progress) => {
                self.postMessage({
                    type: 'progress',
                    progress: progress.progress,
                    total: progress.total,
                    unique: progress.unique
                });
            });
        } else {
            result = count_tokens_from_raw_text(input, RAW_TEXT_LIMIT);
        }
        
        self.postMessage({
            type: 'result',
            total: result.total,
            unique: result.unique
        });
    } catch (error) {
        self.postMessage({
            type: 'error',
            message: error.toString()
        });
    }
}

async function updateRegex(pattern) {
    try {
        await set_token_regex(pattern);
        self.postMessage({type: 'regex_updated'});
    } catch (error) {
        self.postMessage({
            type: 'error',
            message: error.toString()
        });
    }
}

self.onmessage = async function (e) {
    const msg = e.data;

    switch (msg.type) {
        case 'process_text':
            await processInput(msg.text || msg.input);
            break;
        case 'set_regex':
            await updateRegex(msg.pattern);
            break;
        default:
            self.postMessage({
                type: 'error',
                message: 'Unknown command'
            });
    }
};