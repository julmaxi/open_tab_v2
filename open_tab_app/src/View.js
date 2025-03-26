//@ts-check
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { useState, useEffect, useRef } from "react";
import _ from 'lodash';

export function useView(viewDef, defaultVal) {
    let [view, setView] = useState(defaultVal);

    let viewId = useRef(-1);

    useEffect(() => {
        if (view) {
            setView({
                ...view,
                isLoading: true
            })
        }
        invoke("subscribe_to_view", {view: viewDef}).then((msg) => {
            if (msg["success"] === undefined) {
                console.log("Error", msg);
            }
            else {
                let viewResult = JSON.parse(msg["success"]["content"]);

                if (viewId !== null) {
                    setView(viewResult);    
                    viewId.current = msg["success"]["ref_id"];
                }
                else {
                    invoke("unsubscribe_from_view", {refId: viewId.current});
                }
            }
        });

        return () => {
            if (viewId.current !== -1) {
                invoke("unsubscribe_from_view", {refId: viewId.current});
                viewId.current = null;
            }
        }
    }, [...Object.values(viewDef)]);

    useEffect(
        () => {
            const unlisten = listen('views-changed', (event) => {

            let relevant_changes = event.payload.changes.filter((change) => _.isEqual(change.view, viewDef));

            if (relevant_changes.length > 0) {
                let updatedPaths = relevant_changes[0].updated_paths;
                let new_view = {...view};
                for (var change_path in updatedPaths) {
                    console.log("Change path", change_path);
                    if (change_path === ".") {
                        new_view = updatedPaths[change_path];
                    }
                    else {
                        let parsed_change_path = change_path.split(".").map(e => !isNaN(parseInt(e)) ? parseInt(e) : e);
                        new_view = updatePath(new_view, parsed_change_path, updatedPaths[change_path])    
                    }
                }
                setView(new_view);
            }
            })

            return () => {
                unlisten.then((unlisten) => unlisten())
            }
        },
        [view]
    );

    return view;
}



export function getPath(obj, path) {
    return path.reduce((acc, part) => acc[part], obj);
}
  
  
export function clone(e) {
    return structuredClone(e);
}
  
export function updatePath(obj, path, new_val) {
    if (path.length == 0) {
      return new_val;
    }
    let child = obj[path[0]];
  
    let val = updatePath(child, path.slice(1), new_val)
    if (Array.isArray(obj)) {
        let new_obj = [...obj];
        new_obj[path[0]] = val;
        return new_obj;
    }
    else {
        return {...obj, [path[0]]: val};
    }
}
