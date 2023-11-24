import { invoke } from "@tauri-apps/api/tauri";


export async function executeAction(type, params) {
  let result = await invoke("execute_action", {
    action: {
      type: type,
      action: params
    }
  });

  if (result.success == true) {
    return true;
  }
  else {
    console.error(result, type, params);
    return false;
  }
}
