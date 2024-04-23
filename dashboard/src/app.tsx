import { createRoot } from 'react-dom/client';
import { Menu } from './components/menu';

import 'rsuite/dist/rsuite.min.css';
import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { Tasks } from './components/tasks';
import { UserProvider } from './contexts/user';
import { Content } from './content';
import { CustomProvider } from 'rsuite';
import { AuthProvider } from 'react-oidc-context';


const App = () => {

    const oidcConfig = {
        authority: "https://id.lab.wobcom.de/realms/wobcom/",
        client_id: "vicky-dev", 
        redirect_uri: "http://localhost:1234", 
        onSigninCallback: (): void => {
          window.history.replaceState({}, document.title, window.location.pathname);
        },
    };

    return (
        <>
         <CustomProvider theme="dark">
            <AuthProvider {...oidcConfig}>
                <BrowserRouter>
                    <Content></Content>
                </BrowserRouter>
            </AuthProvider>
        </CustomProvider>
        </>
    )

}


const root = createRoot(document.getElementById('root')!);
root.render(<App />);
