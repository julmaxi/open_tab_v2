import React, { useState } from 'react';

import ModalOverlay from './Modal';

const RemoteSelector = ({ knownRemotes, currentRemoteUrl, onSetRemote }) => {
  const [isOpen, setIsOpen] = useState(false);
  const currentRemote = knownRemotes.find(remote => remote.url === currentRemoteUrl);

  const handleOpenModal = () => {
    setIsOpen(true);
  };

  const handleRemoteSelected = (remote) => {
    console.log(remote);
    onSetRemote(remote.url)
    setIsOpen(false);
  };

  const handleAbort = () => {
    setIsOpen(false);
  };

  return (
    <div className="flex items-center border p-1 rounded-md">
      <span className={`flex-grow ${currentRemote ? "font-medium text-sm" : "text-gray-400 text-xs"}`}>
        {currentRemote ? currentRemote.name : 'No remote'}
      </span>
      <button 
        onClick={handleOpenModal}
        className="bg-blue-500 text-white px-2 py-1 text-xs rounded-md"
      >
        Select
      </button>

      {isOpen && (
        <ModalOverlay open={isOpen}>
          <div className="bg-white p-3 rounded-md max-w-xs">
            <h2 className="text-xs font-medium mb-2">Select a Remote</h2>
            <ul className="space-y-1">
              {knownRemotes.map((remote) => (
                <li key={remote.url}>
                  <button
                    onClick={() => handleRemoteSelected(remote)}
                    className="text-left p-1 hover:bg-gray-200 rounded-md block w-full"
                  >
                    {remote.name}
                  </button>
                </li>
              ))}
            </ul>
            <button 
              onClick={handleAbort} 
              className="mt-2 bg-red-500 text-white p-1 rounded-md w-full text-xs"
            >
              Abort
            </button>
          </div>
        </ModalOverlay>
      )}
    </div>
  );
};

export default RemoteSelector;
