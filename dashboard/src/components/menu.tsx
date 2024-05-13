import { Nav, Navbar } from "rsuite"

import * as s from "./menu.module.css";

import UserIcon from '@rsuite/icons/legacy/User';

import { Link, useNavigate } from "react-router-dom";
import { useContext, useEffect, useState } from "react";
import { useAPI } from "../services/api";
import { UserContext } from "../contexts/user";
import { useAuth } from "react-oidc-context";


const Menu = () => {

    const activeKey = "tasks";
    const user = useContext(UserContext)
    const auth = useAuth();

    return (
        <Navbar bordered shaded>
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
            </Nav>
        </Navbar>
    )


}

export {
    Menu
}