import React, { useState } from "react";

export function TabGroup({...props}) {
  let [activeTab, setActiveTab] = useState(0);

  let children = React.Children.toArray(props.children);
  let displayChild = children[activeTab];

  return <div className="flex flex-col w-full h-full">
    <div className="flex">
      {React.Children.map(props.children, (tab, i) => <button className={"flex-1 text-center p-2 font-semibold text-sm" + (i == activeTab ? " bg-blue-500 text-white" : " bg-gray-100")} onClick={() => setActiveTab(i)}>{tab.props.name}</button>)}
    </div>
    {displayChild}
  </div>;
}

export function Tab({...props}) {
  return <div className={`min-h-0 flex-1 ${props.autoScroll !== false ? "overflow-scroll" : ""}`}>
    {props.children}
  </div>;
}
