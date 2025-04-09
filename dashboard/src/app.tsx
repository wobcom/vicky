import { createRoot } from 'react-dom/client';

import 'rsuite/dist/rsuite.min.css';
import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { Content } from './content';
import { CustomProvider } from 'rsuite';
import { AuthProvider } from 'react-oidc-context';
import { WebConfigContext, WebConfigProvider } from './contexts/web-config';

import * as dayjs from "dayjs";
import * as relativeTime from 'dayjs/plugin/relativeTime'

dayjs.extend(relativeTime);

const App = () => {
    return (
        <WebConfigProvider>
            <CustomProvider theme="dark">
                <WebConfigContext.Consumer>
                    {value => {
                        const url = new URL(window.location.href)
                        const oidcConfig = {
                            authority: value?.authority,
                            client_id: value?.client_id,
                            redirect_uri: `${url.protocol}//${url.host}`,
                            scope: "openid profile email",
                            onSigninCallback: (): void => {
                                window.history.replaceState({}, document.title, window.location.pathname);
                            },
                        };

                        return (
                            <AuthProvider {...oidcConfig}>
                                <BrowserRouter>
                                    <Content></Content>
                                </BrowserRouter>
                            </AuthProvider>
                        )
                    }}
                </WebConfigContext.Consumer>
            </CustomProvider>
        </WebConfigProvider>
    )
}


const root = createRoot(document.getElementById('root')!);
root.render(<App />);
