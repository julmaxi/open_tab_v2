import React from 'react';
import LoginWidget from './LoginWidget';

function LoginWindow() {
    return (
        <div className="flex justify-center items-center h-screen bg-gray-200">
            <LoginWidget
                defaultUserName="johnDoe"
                onLogin={(username, password) => {
                    console.log("Logging in with", username, password);
                }}
                onAbort={() => {
                    console.log("Login aborted");
                }}
            />
        </div>
    );
}

export default LoginWindow;
