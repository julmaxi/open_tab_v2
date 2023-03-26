import { invoke } from "@tauri-apps/api/tauri";


export function executeAction(type, params) {
    invoke("execute_action", {
    action: {
      type: type,
      action: params
    }
  }).then((msg) => {
  });
}
