import React, { useState } from 'react';

import ModalOverlay from './Modal';
import { executeAction } from './Action';
import { invoke } from '@tauri-apps/api/tauri';
import { confirm } from '@tauri-apps/api/dialog';


const RemotesList = ({ knownRemotes, onRemoteSelected, onAbort }) => {
  let [isAddingRemote, setIsAddingRemote] = useState(false);

  const handleAddRemote = () => {
    setIsAddingRemote(true);
  }

  const handleRemoteAdded = (remote) => {
    invoke("add_remote", {newRemote: remote});
    setIsAddingRemote(false);
  }

  if (isAddingRemote) {
    return (
      <AddRemoteForm
        onRemoteAdded={handleRemoteAdded}
        onAbort={() => setIsAddingRemote(false)}
      />
    );
  }

  return (
    <div className="bg-white p-3 rounded-md max-w-xs">
      <h2 className="text-xs font-medium mb-2">Select a Remote</h2>
      <ul className="space-y-1">
        {knownRemotes.map((remote) => (
          <li key={remote.url} className='flex'>
            <button
              onClick={() => onRemoteSelected(remote)}
              className="text-left p-1 hover:bg-gray-200 rounded-md w-full flex justify-between items-center"
            >
              <span>
                {remote.name}
                <p className="text-xs text-gray-400">{remote.url}</p>
              </span>
            </button>
            <button
              onClick={() => {
                confirm("Are you sure you want to delete this participant? You can not undo this.").then((result) => {
                  if (result === true) {
                    invoke("remove_remote", {url: remote.url});
                  }
                })
              }}
              className=""
              style={{outline: 'none'}}
            >
              ✕ {/* This is a simple cross symbol, you might want to use an icon instead */}
            </button>
          </li>
        ))}
      </ul>
      <button
        onClick={handleAddRemote}
        className="mt-2 bg-blue-500 text-white p-1 rounded-md w-full text-xs"
      >
        Add New Remote
      </button>
      <button 
        onClick={onAbort} 
        className="mt-2 bg-red-500 text-white p-1 rounded-md w-full text-xs"
      >
        Abort
      </button>

    </div>
  );
};


const AddRemoteForm = ({ onRemoteAdded, onAbort }) => {
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');

  const handleAddRemote = () => {
    onRemoteAdded({ name, url });
  };

  return (
    <div className="bg-white p-3 rounded-md max-w-xs">
      <h2 className="text-xs font-medium mb-2">Add a Remote</h2>
      <div className="space-y-2">
        <div>
          <label className="text-xs" htmlFor="name">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="border rounded-md p-1 w-full"
          />
        </div>
        <div>
          <label className="text-xs" htmlFor="url">URL</label>
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            className="border rounded-md p-1 w-full"
          />
        </div>
      </div>
      <div className="flex justify-between pt-2">
        <button
          onClick={onAbort}
          className="bg-red-500 text-white p-1 rounded-md w-24 text-xs"
        >
          Abort
        </button>
        <button
          onClick={handleAddRemote}
          className="bg-blue-500 text-white p-1 rounded-md w-24 text-xs"
        >
          Add
        </button>
      </div>
    </div>
  );
};

const RemoteSelector = ({ knownRemotes, currentRemoteUrl, onSetRemote }) => {
  const [isOpen, setIsOpen] = useState(false);
  const currentRemote = knownRemotes.find(remote => remote.url === currentRemoteUrl);

  const handleOpenModal = () => {
    setIsOpen(true);
  };

  const handleRemoteSelected = (remote) => {
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
          <RemotesList
            knownRemotes={knownRemotes}
            onRemoteSelected={handleRemoteSelected}
            onAbort={handleAbort}
          />
        </ModalOverlay>
      )}
    </div>
  );
};

export default RemoteSelector;
