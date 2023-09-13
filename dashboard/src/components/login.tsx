import { Button, Col, FlexboxGrid, Panel, Stack } from "rsuite"

import GitHubIcon from '@rsuite/icons/legacy/Github';
import * as s from "./login.module.css";


const Login = () => {

    return (
        <FlexboxGrid
            align="middl;e"
            justify="center"
        >
            <Col xs={8}>
                <Panel shaded bordered bodyFil>
                <Stack spacing={16} direction="column">
                    <h2>Login via GitHub</h2>
                    <p className={s.ExplanationText}>
                            Ein Login ist momentan nur über GitHub möglich. Zusätzlich muss der GitHub-Account für Vicky freigeschaltet sein.
                            Wenn der Account nicht freigeschaltet ist, ist ein Einloggen nicht möglich.
                    </p>

                    <Button href={"/api/auth/login/github"} color="violet" appearance="primary" startIcon={<GitHubIcon />}>
                        Login With GitHub
                    </Button>
                </Stack>
                </Panel>
            </Col>

        </FlexboxGrid>
    )
}

export {
    Login
}