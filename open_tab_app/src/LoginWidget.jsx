// LoginWidget.js

import React, { useState } from 'react';

function getMessageForError(error) {
    if (error.error === "IncorrectPassword") {
        return "Username or password incorrect.";
    }
    else if (error.error === "UserExists") {
        return "Username already taken.";
    }
    else if (error.error === "PasswordTooShort") {
        return "Password must be at least 8 characters long.";
    }
    else {
        return "Unknown error: " + error.error;
    }
}

function LoginWidget({ onLogin, onAbort, onAccountCreation, url, loginError, defaultUserName = "" }) {
    const [username, setUsername] = useState(defaultUserName);
    const [password, setPassword] = useState("");
    const [repeatPassword, setRepeatPassword] = useState("");
    const [mode, setMode] = useState("login"); // Either 'login' or 'createAccount'

    let errMsg = loginError && getMessageForError(loginError);
    console.log(errMsg);

    return (
        <div className="bg-gray-100 p-6 rounded shadow-md w-96">
            <h2 className="text-2xl font-semibold mb-4">{mode === 'login' ? `Login to ${url}` : 'Create Account'}</h2>

            
            {loginError && <p className='text-red-500 font-bold'>{errMsg}</p>}
            

            <div className="mb-4">
                <label className="block text-sm font-medium mb-2" htmlFor="username">
                    Username
                </label>
                <input
                    id="username"
                    type="text"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    className="p-2 border rounded w-full"
                />
            </div>

            <div className="mb-4">
                <label className="block text-sm font-medium mb-2" htmlFor="password">
                    Password
                </label>
                <input
                    id="password"
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="p-2 border rounded w-full"
                />
            </div>

            {mode === 'createAccount' && (
                <div className="mb-4">
                    <label className="block text-sm font-medium mb-2" htmlFor="repeatPassword">
                        Repeat Password
                    </label>
                    <input
                        id="repeatPassword"
                        type="password"
                        value={repeatPassword}
                        onChange={(e) => setRepeatPassword(e.target.value)}
                        className="p-2 border rounded w-full"
                    />
                </div>
            )}

            <div className="flex justify-between items-center">
                <button
                    onClick={() => onAbort()}
                    className="text-red-600 hover:text-red-800"
                >
                    Abort
                </button>
                <div>
                    {mode === 'login' && (
                        <button
                            onClick={() => setMode('createAccount')}
                            className="mr-2 bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700"
                        >
                            Create Account
                        </button>
                    )}
                    <button
                        onClick={() => {
                            if (mode === 'login') {
                                onLogin(username, password);
                            } else {
                                onAccountCreation(username, password);
                            }
                        }}
                        className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
                    >
                        {mode === 'login' ? 'Login' : 'Register'}
                    </button>
                </div>
            </div>
        </div>
    );
}

export default LoginWidget;