import React, { useState } from "react";

export function TabGroup({initialActiveTab = 0, ...props}) {
  let [activeTab, setActiveTab] = useState(undefined);

  let children = React.Children.toArray(props.children);
  let reallyActiveTab = activeTab === undefined ? initialActiveTab : activeTab;
  let displayChild = children[reallyActiveTab];

  return <div className="flex flex-col w-full h-full">
    <div className="flex">
      {children.map((tab, i) => <button key={i} className={"flex-1 text-center p-2 font-semibold text-sm" + (i == reallyActiveTab ? " bg-blue-500 text-white" : " bg-gray-100")} onClick={() => setActiveTab(i)}>{tab.props.name}</button>)}
    </div>
    {displayChild}
  </div>;
}

export function Tab({...props}) {
  return <div className={`min-h-0 flex-1 ${props.autoScroll !== false ? "overflow-auto" : ""}`}>
    {props.children}
  </div>;
}
