import { Nav, Navbar } from "rsuite"

import * as s from "./menu.module.css";

import UserIcon from '@rsuite/icons/legacy/User';

import { Link, useNavigate } from "react-router-dom";
import { useContext, useEffect, useState } from "react";
import { useAPI } from "../services/api";
import { UserContext } from "../contexts/user";


const Menu = () => {

    const activeKey = "tasks";
    const user = useContext(UserContext)

    return (
        <Navbar bordered shaded className={s.Navbar}>
            <Navbar.Brand>
                <Link to={"/"}>Vicky</Link>
            </Navbar.Brand>
            {
                user ? (
                    <Nav activeKey={activeKey}>
                        <Nav.Item eventKey="tasks" >
                            <Link to={"/tasks"}>Tasks</Link>
                        </Nav.Item>
                    </Nav>
                )
                : null
            }
            <Nav pullRight>
                { user ? (
                    <Nav.Item icon={<UserIcon />}>
                        {user.full_name}
                    </Nav.Item>
                ) : null}
                
                { !user ? (
                    <Nav.Item href={"/api/auth/login/github"}>
                    Login With GitHub
                </Nav.Item>
                ): null}
                
            </Nav>
        </Navbar>
    )


}

export {
    Menu
}