import { Navigate, Route, Routes } from "react-router-dom"
import { Login } from "./components/login"
import { Menu } from "./components/menu"
import { Tasks } from "./components/tasks"
import { UserContext, UserProvider } from "./contexts/user"

import * as s from "./content.module.css"
import { useAuth } from "react-oidc-context"
import * as React from "react";


const Content = () => {
    const auth = useAuth()

    switch (auth.activeNavigator) {
        case "signinSilent":
            return <div>Signing you in...</div>;
        case "signoutRedirect":
            return <div>Signing you out...</div>;
    }

    if (auth.isLoading) {
        return <div>Loading...</div>;
    }

    if (auth.error) {
        return <div>Oops... {auth.error.message}</div>;
    }

    if (auth.isAuthenticated) {
        return (
            <UserProvider>
                <Menu></Menu>
                <div className={s.Content}>
                    <Routes>
                        <Route path="/tasks" Component={Tasks} />
                        <Route path="/tasks/:taskId" Component={Tasks} />
                        <Route path="/" element={<Navigate replace to="/tasks" />}>
                        </Route>
                    </Routes>
                </div>

            </UserProvider>
        );
    } else {
        return (
            <>
                <Menu></Menu>
                <div className={s.Content}>
                    <Routes>

                        <Route path="*" Component={Login} />
                    </Routes>
                </div>
            </>
        )
    }

}

export {
    Content
}