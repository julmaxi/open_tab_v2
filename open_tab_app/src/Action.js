import { invoke } from "@tauri-apps/api/tauri";
import { createContext, useContext } from 'react';


export const ErrorHandlingContext = createContext({handleError: (error) => {}});


/**
 * 
 * @param {*} type 
 * @param {*} params 
 * @param {*} handleError 
 * @returns 
 */
export async function executeAction(type, params, handleError = null) {
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
    if (handleError !== null) {
      handleError(result.error);
    }
    return false;
  }
}
