import { createRoot } from 'react-dom/client';
import { Menu } from './components/menu';

import 'rsuite/dist/rsuite.min.css';
import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { Tasks } from './components/tasks';
import { UserProvider } from './contexts/user';
import { Content } from './content';
import { CustomProvider } from 'rsuite';


const App = () => {

    return (
        <>
         <CustomProvider theme="dark">
            <UserProvider>
                <BrowserRouter>
                    <Content></Content>
                </BrowserRouter>
            </UserProvider>
        </CustomProvider>
        </>
    )

}


const root = createRoot(document.getElementById('root')!);
root.render(<App />);
