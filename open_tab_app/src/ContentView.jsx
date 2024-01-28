import React, { useState } from 'react';

const ContentView = ({ children, forceOpen, defaultDrawerWidth: defaultDrawerWidth = 256 }) => {
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);

  const toggleDrawer = () => {
    setIsDrawerOpen(!isDrawerOpen);
  };

  let subcomponents = {};
  React.Children.forEach(children, (child) => {
    subcomponents[ContentView.SubcomponentNames[child.type.name]] = child;
  });

  return (
    <div className="flex h-full relative">
      <div className="flex flex-1 min-w-0">
        {subcomponents.Content || ""}
      </div>
      <button 
        className="absolute top-2 right-2 text-black rounded-md bg-white"
        onClick={toggleDrawer}
      >
            {
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-5 h-5">
                <path fillRule="evenodd" d="M2 4.75A.75.75 0 012.75 4h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 4.75zM2 10a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 10zm0 5.25a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75a.75.75 0 01-.75-.75z" clipRule="evenodd" />
            </svg>
            }
      </button>
      {(isDrawerOpen || forceOpen) && <div style={{width: defaultDrawerWidth.toString() + "px"}} className="bg-gray-100 border-l">{
        subcomponents.Drawer || ""
      }</div>}
    </div>
  );
};

let Content = ({ children }) => {
    return <div className="flex-1 min-w-0">{children}</div>;
};

const Drawer = ({ children, size = 'w-64', isOpen = true }) => {
    return (
        isOpen && <div className={`${size} bg-gray-100 border-l w-full h-full`}>{children}</div>
    );
};

ContentView.Content = Content;
ContentView.SubcomponentNames = {
    [Content.name]: "Content",
    [Drawer.name]: "Drawer"
};
ContentView.Drawer = Drawer;
ContentView.DrawerKey = Drawer.name;



export default ContentView;