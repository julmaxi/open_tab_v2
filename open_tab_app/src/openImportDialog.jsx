import { open } from '@tauri-apps/api/dialog';
import { invoke } from "@tauri-apps/api/tauri";


export async function openImportDialog() {
    const selected = await open({
        multiple: false,
        filters: [{
            name: 'csv',
            extensions: ['csv']
        }]
    });

    if (selected !== null) {
        let proposedConfig = await invoke("guess_csv_config", { path: selected });
        return {
            file: selected,
            proposedConfig
        };
    }
    else {
        return null;
    }
}
