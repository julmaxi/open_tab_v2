import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useState, useEffect } from "react";

export function useSettings() {
    let [settings, setSettings] = useState({});

    useEffect(() => {
        invoke("get_settings").then((msg) => {
            setSettings(msg);    
        });

        const unlisten = listen('settings-changed', (event) => {
            console.log(event);
            setSettings(event.payload);
        });

        return () => {
            unlisten.then((unlisten) => unlisten())
        }
    }, []);

    return settings;
}