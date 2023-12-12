import { useState } from "react";

import { listen } from "@tauri-apps/api/event";


function ProgressBar({progress, ...props}) {
    let style = {
        width: progress !== null ? `${progress * 100}%` : "0%",
    }
    return <div className="h-5 rounded w-full bg-gray-400 flex-col">
        <div className="h-0">
        <div className="h-5 bg-blue-800 rounded" style={style}></div>
        </div>

        <div className="text-center w-full h-5 text-white font-bold flex justify-center items-center text-sm">{Math.round(progress * 100)}%</div>
    </div>
}


export default function UpdateProgressWindow() {
    let [progress, setProgress] = useState(null);

    listen("update-download-progress", (event) => {
        setProgress(event.payload.progress);
    });

    return <div className="w-screen h-screen flex flex-col justify-center items-center p-10">
        <p>Downloading Update…</p>
        <ProgressBar progress={progress} />
    </div>
}