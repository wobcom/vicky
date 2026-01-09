import {Avatar, Nav, Navbar} from "rsuite"

import * as s from "./menu.module.css";

import UserIcon from '@rsuite/icons/legacy/User';

import {Link, useNavigate} from "react-router-dom";
import {useContext, useEffect, useState} from "react";
import {useAPI} from "../services/api";
import {UserContext} from "../contexts/user";
import {useAuth} from "react-oidc-context";
import AvatarIcon from "rsuite/esm/Avatar/AvatarIcon";


const Menu = () => {

    const user = useContext(UserContext)
    const auth = useAuth();

    return (
        <Navbar>
            <Navbar.Content>
                <Navbar.Brand>
                    <Link to={"/"}>Vicky</Link>
                </Navbar.Brand>
                {
                    user ? (
                                <Nav>
                                    <Nav.Item eventKey="tasks">
                                        <Link to={"/tasks"}>Tasks</Link>
                                    </Nav.Item>
                                </Nav>
                        )
                        : null
                }
            </Navbar.Content>
            <Navbar.Content>
                <Nav>
                    {user ? (
                        <Nav.Item icon={<Avatar circle background={"transparent"} size="sm" />}>
                            {user.full_name}
                        </Nav.Item>
                    ) : null}
                </Nav>
            </Navbar.Content>
        </Navbar>
    )


}

export {
    Menu
}