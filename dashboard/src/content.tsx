import { useContext } from "react"
import { Navigate, Route, Routes } from "react-router-dom"
import { Login } from "./components/login"
import { Menu } from "./components/menu"
import { Tasks } from "./components/tasks"
import { UserContext } from "./contexts/user"

import * as s from "./content.module.css"


const Content = () => {

    const user = useContext(UserContext)

    return (
        <>
            <Menu></Menu>
            <div className={s.Content}>

            {
                user ? (
                    <Routes>
                        <Route path="/tasks" Component={Tasks} />
                        <Route path="/tasks/:taskId" Component={Tasks} />
                        <Route path="/" element={<Navigate replace to="/tasks" />}>
                        </Route>
                    </Routes>
    
                ) : (
                    <Routes>
                        <Route path="*" Component={Login} />
                    </Routes>
                )

            }
            </div>
        </>
    )
}

export {
    Content
}