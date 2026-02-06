import {Avatar, Nav, Navbar} from "rsuite"

import * as s from "./menu.module.css";

import {NavLink} from "react-router-dom";
import {ReactEventHandler, useContext} from "react";
import {UserContext} from "../contexts/user";
import {useAuth} from "react-oidc-context";
import * as React from "react";

const Menu = () => {
    const user = useContext(UserContext)
    const auth = useAuth();

    return (
        <Navbar>
            <Navbar.Content>
                <Navbar.Brand as={NavLink} to="/">
                    Vicky
                </Navbar.Brand>
                {
                    user ? (
                            <Nav>
                                <Nav.Item as={NavLink} to="/tasks">Tasks</Nav.Item>
                            </Nav>
                        )
                        : null
                }
            </Navbar.Content>
            <Navbar.Content>
                <Nav>
                    {user ? (
                        <Nav.Item icon={<Avatar circle background={"transparent"} size="sm"/>}>
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