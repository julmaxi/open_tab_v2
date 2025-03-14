import { ask } from "@tauri-apps/api/dialog";
import { useContext, createContext } from 'react';
import { ErrorHandlingContext, executeAction } from "../../Action";


export const DrawEditorSettingsContext = createContext({
    showMiscIssues: true,
    showLowIssues: true,
    showMidIssues: true,
    showHighIssues: true,
    enableAlterTeamDraw: false,
    updateSettings: (newSettings) => { }
});


export function DrawSettingsEditor({ round_id }) {
    let settings = useContext(DrawEditorSettingsContext);
    let errorContext = useContext(ErrorHandlingContext);

    const handleShowMiscIssuesChange = () => {
        settings.updateSettings({ ...settings, showMiscIssues: !settings.showMiscIssues });
    };

    const handleShowLowIssuesChange = () => {
        settings.updateSettings({ ...settings, showLowIssues: !settings.showLowIssues });
    };

    const handleShowMidIssuesChange = () => {
        settings.updateSettings({ ...settings, showMidIssues: !settings.showMidIssues });
    };

    const handleShowHighIssuesChange = () => {
        settings.updateSettings({ ...settings, showHighIssues: !settings.showHighIssues });
    };

    const handleSetEnableAlterTeamDraw = () => {
        settings.updateSettings({ ...settings, enableAlterTeamDraw: !settings.enableAlterTeamDraw });
    }

    return (
        <div className="p-4">
            <div className="space-y-4">
                <div>
                    <label className="flex items-center space-x-2 text-sm font-medium text-gray-700">
                        <input
                            type="checkbox"
                            className="form-checkbox rounded text-blue-600 focus:border-blue-300 focus:ring focus:ring-blue-200 focus:ring-opacity-50"
                            checked={settings.showMiscIssues}
                            onChange={handleShowMiscIssuesChange}
                        />
                        <span>Show Misc. Issues</span>
                    </label>
                </div>
                <div>
                    <label className="flex items-center space-x-2 text-sm font-medium text-blue-500">
                        <input
                            type="checkbox"
                            className="form-checkbox rounded text-blue-600 focus:border-blue-300 focus:ring focus:ring-blue-200 focus:ring-opacity-50"
                            checked={settings.showLowIssues}
                            onChange={handleShowLowIssuesChange}
                        />
                        <span>Show Low Issues</span>
                    </label>
                </div>
                <div>
                    <label className="flex items-center space-x-2 text-sm font-medium text-yellow-700">
                        <input
                            type="checkbox"
                            className="form-checkbox rounded text-yellow-600 focus:border-yellow-300 focus:ring focus:ring-yellow-200 focus:ring-opacity-50"
                            checked={settings.showMidIssues}
                            onChange={handleShowMidIssuesChange}
                        />
                        <span>Show Mid Issues</span>
                    </label>
                </div>
                <div>
                    <label className="flex items-center space-x-2 text-sm font-medium text-red-700">
                        <input
                            type="checkbox"
                            className="form-checkbox rounded text-red-600 focus:border-red-300 focus:ring focus:ring-red-200 focus:ring-opacity-50"
                            checked={settings.showHighIssues}
                            onChange={handleShowHighIssuesChange}
                        />
                        <span>Show Severe Issues</span>
                    </label>
                </div>
            </div>

            <div className="mt-4">
                <label className="flex items-center space-x-2 text-sm font-medium">
                    <input
                        type="checkbox"
                        className="form-checkbox rounded focus:ring focus:ring-opacity-50"
                        checked={settings.enableAlterTeamDraw}
                        onChange={handleSetEnableAlterTeamDraw}
                    />
                    <span>Allow Reassiging Teams/Non Aligned</span>
                </label>
            </div>

            <div className="mt-4">
                <button
                    className="bg-blue-500 hover:bg-blue-600 text-white py-1 px-2 rounded"
                    onClick={() => {
                        ask('Are you sure? This will override the previous venues.', { title: 'Regenerate Break', type: 'warning' }).then(
                            (result) => {
                                console.log(result);
                                if (result === true) {
                                    executeAction("RedrawRound", { round_id: round_id, mode: "Venues" }, errorContext.handleError);
                                }
                            })
                    }}
                >
                    Assign Venues
                </button>
                <button
                    className="bg-blue-500 hover:bg-blue-600 text-white py-1 px-2 rounded"
                    onClick={() => {
                        ask('Are you sure? This will override part of the draw.', { title: 'Regenerate Break', type: 'warning' }).then(
                            (result) => {
                                console.log(result);
                                if (result === true) {
                                    executeAction("RedrawRound", { round_id: round_id, mode: "MissingNonAligned" }, errorContext.handleError);
                                }
                            })
                    }}
                >
                    Assign Missing Non-Aligned
                </button>
            </div>
        </div>
    );
}