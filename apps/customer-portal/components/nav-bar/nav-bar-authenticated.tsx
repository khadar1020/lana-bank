"use client"
import React, { useState } from "react"
import Link from "next/link"

import { IoPersonOutline, IoCloseOutline } from "react-icons/io5"

import { LavaBankIcon } from "../icons"
import { Card, CardContent, CardHeader, CardTitle } from "../primitive/card"
import { Button } from "../primitive/button"
import { Key, KeyValueCell, KeyValueGroup, Value } from "../primitive/aligned-key-value"

import useLogout from "@/hooks/use-logout"

export type NavBarAuthenticatedProps = {
  email: string
  twoFactorEnabled: boolean
}

export function NavBarAuthenticated({
  email,
  twoFactorEnabled,
}: NavBarAuthenticatedProps) {
  const [openMenu, setOpenMenu] = useState(false)
  const { logout } = useLogout()
  return (
    <>
      <nav
        className={`max-w-[70rem] m-auto flex justify-between items-center mt-2 relative `}
      >
        <div className="flex items-center gap-4">
          <Link href="/">
            <LavaBankIcon />
          </Link>
          <p className="mt-4">Bitcoin Backed Loans</p>
        </div>
        <div className="flex items-center gap-4 p-4">
          <p>{email}</p>
          <div
            onClick={() => setOpenMenu(true)}
            className="border border-primary p-2 rounded-full cursor-pointer"
          >
            <IoPersonOutline className="w-6 h-6" />
          </div>
          {openMenu && (
            <div className="absolute right-0 top-0 z-20">
              <Card className="w-80 border-none">
                <div className=" flex justify-between items-center p-4">
                  <Button
                    variant="ghost"
                    className="p-1"
                    onClick={() => {
                      setOpenMenu(false)
                    }}
                  >
                    <IoCloseOutline className="w-6 h-6 " />
                  </Button>
                  <div className="flex justify-end items-center gap-4">
                    <p>{email}</p>
                    <div className="border border-primary p-2 rounded-full">
                      <IoPersonOutline className="w-6 h-6" />
                    </div>
                  </div>
                </div>
                <Card variant="transparent">
                  <CardHeader className="pt-0 pb-4">
                    <CardTitle>Account</CardTitle>
                  </CardHeader>
                  <CardContent className="p-6 pt-0 flex flex-col gap-2 text-sm">
                    <KeyValueGroup>
                      <KeyValueCell>
                        <Key>Email</Key>
                        <Value>{email}</Value>
                      </KeyValueCell>
                      {twoFactorEnabled && (
                        <KeyValueCell>
                          <Key>Two Factor Authentication</Key>
                          <Value>Enabled</Value>
                        </KeyValueCell>
                      )}
                    </KeyValueGroup>
                    <Button onClick={async () => logout()} className="mt-4">
                      Logout
                    </Button>
                  </CardContent>
                </Card>
              </Card>
            </div>
          )}
        </div>
      </nav>
      {openMenu && (
        <div
          onClick={() => setOpenMenu(false)}
          className="fixed inset-0 bg-black bg-opacity-65 z-10"
        />
      )}
    </>
  )
}
