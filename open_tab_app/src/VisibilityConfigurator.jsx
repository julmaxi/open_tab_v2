import React from "react";

export const VisibilityConfigurator = ({
    visibility, onUpdateVisibility
}) => {
    // Group configurations by target groups (the "x" in "y_for_x")
    const groupedConfig = {
        chairs: {
            title: "Target: Chairs",
            options: {
                show_wings_for_chairs: "Show Wings to Chairs",
                show_presidents_for_chairs: "Show Presidents to Chairs",
                show_teams_for_chairs: "Show Teams to Chairs",
                show_non_aligned_for_chairs: "Show Non-Aligned to Chairs"
            }
        },
        wings: {
            title: "Target: Wings",
            options: {
                show_chairs_for_wings: "Show Chairs to Wings",
                show_presidents_for_wings: "Show Presidents to Wings",
                show_wings_for_wings: "Show Wings to Wings",
                show_teams_for_wings: "Show Teams to Wings",
                show_non_aligned_for_wings: "Show Non-Aligned to Wings"
            }
        },
        presidents: {
            title: "Target: Presidents",
            options: {
                show_chairs_for_presidents: "Show Chairs to Presidents",
                show_wings_for_presidents: "Show Wings to Presidents",
                show_teams_for_presidents: "Show Teams to Presidents",
                show_non_aligned_for_presidents: "Show Non-Aligned to Presidents"
            }
        }
    };

    const handleToggle = (key) => {
        onUpdateVisibility({
            ...visibility,
            [key]: !visibility[key]
        });
    };

    const handleGroupToggle = (group) => {
        const groupKeys = Object.keys(groupedConfig[group].options);
        const allEnabled = groupKeys.every(key => visibility[key]);

        const newConfig = { ...visibility };
        groupKeys.forEach(key => {
            newConfig[key] = !allEnabled;
        });

        onUpdateVisibility(newConfig);
    };

    const handleReset = () => {
        onUpdateVisibility(Object.keys(visibility).reduce((acc, key) => {
            acc[key] = false;
            return acc;
        }, {}));
    };

    const handleSelectAll = () => {
        onUpdateVisibility(Object.keys(visibility).reduce((acc, key) => {
            acc[key] = true;
            return acc;
        }, {}));
    };

    return (
        <div className="max-w-4xl mx-auto p-6 bg-white rounded-lg shadow-md">
            <h2 className="text-2xl font-bold mb-6 text-gray-800">Feedback Visibility</h2>

            <div className="mb-6 flex space-x-4">
                <button
                    onClick={handleSelectAll}
                    className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                >
                    Select All
                </button>
                <button
                    onClick={handleReset}
                    className="px-4 py-2 bg-gray-200 text-gray-800 rounded hover:bg-gray-300 transition-colors"
                >
                    Reset All
                </button>
            </div>

            <div className="border rounded-lg overflow-hidden">
                {/* Chairs first */}
                <div className="p-4 bg-gray-50">
                    <div className="flex justify-between items-center mb-4">
                        <h3 className="text-xl font-semibold text-gray-700">{groupedConfig.chairs.title}</h3>
                        <button
                            onClick={() => handleGroupToggle('chairs')}
                            className="text-sm px-3 py-1 bg-blue-100 text-blue-800 rounded hover:bg-blue-200 transition-colors"
                        >
                            Toggle All
                        </button>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                        {Object.entries(groupedConfig.chairs.options).map(([key, label]) => (
                            <div key={key} className="flex items-center">
                                <label className="flex items-center cursor-pointer">
                                    <input
                                        type="checkbox"
                                        checked={visibility[key]}
                                        onChange={() => handleToggle(key)}
                                        className="w-5 h-5 text-blue-600 rounded focus:ring-2 focus:ring-blue-500" />
                                    <span className="ml-2 text-gray-700">{label}</span>
                                </label>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Divider */}
                <div className="border-t border-gray-200"></div>

                {/* Wings second */}
                <div className="p-4 bg-gray-50">
                    <div className="flex justify-between items-center mb-4">
                        <h3 className="text-xl font-semibold text-gray-700">{groupedConfig.wings.title}</h3>
                        <button
                            onClick={() => handleGroupToggle('wings')}
                            className="text-sm px-3 py-1 bg-blue-100 text-blue-800 rounded hover:bg-blue-200 transition-colors"
                        >
                            Toggle All
                        </button>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                        {Object.entries(groupedConfig.wings.options).map(([key, label]) => (
                            <div key={key} className="flex items-center">
                                <label className="flex items-center cursor-pointer">
                                    <input
                                        type="checkbox"
                                        checked={visibility[key]}
                                        onChange={() => handleToggle(key)}
                                        className="w-5 h-5 text-blue-600 rounded focus:ring-2 focus:ring-blue-500" />
                                    <span className="ml-2 text-gray-700">{label}</span>
                                </label>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Divider */}
                <div className="border-t border-gray-200"></div>

                {/* Presidents last */}
                <div className="p-4 bg-gray-50">
                    <div className="flex justify-between items-center mb-4">
                        <h3 className="text-xl font-semibold text-gray-700">{groupedConfig.presidents.title}</h3>
                        <button
                            onClick={() => handleGroupToggle('presidents')}
                            className="text-sm px-3 py-1 bg-blue-100 text-blue-800 rounded hover:bg-blue-200 transition-colors"
                        >
                            Toggle All
                        </button>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                        {Object.entries(groupedConfig.presidents.options).map(([key, label]) => (
                            <div key={key} className="flex items-center">
                                <label className="flex items-center cursor-pointer">
                                    <input
                                        type="checkbox"
                                        checked={visibility[key]}
                                        onChange={() => handleToggle(key)}
                                        className="w-5 h-5 text-blue-600 rounded focus:ring-2 focus:ring-blue-500" />
                                    <span className="ml-2 text-gray-700">{label}</span>
                                </label>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
};
