import { invoke } from "@tauri-apps/api/tauri";
import { useState, useEffect } from "react";

export function useSettings() {
    let [settings, setSettings] = useState({});

    useEffect(() => {
        invoke("get_settings").then((msg) => {
            setSettings(msg);    
        });
    }, []);

    return settings;
}