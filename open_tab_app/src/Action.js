import { invoke } from "@tauri-apps/api/core";
import { createContext, useContext } from 'react';


export const ErrorHandlingContext = createContext({ handleError: (error) => { } });


/**
 * 
 * @param {string} type 
 * @param {*} params 
 * @param {Function} handleError
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
        console.error("Error when executing action", type, result.error);
        if (handleError !== null) {
            handleError(result.error);
        }
        return false;
    }
}
