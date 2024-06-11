"use client"
import React, { useState } from "react"

import dynamic from "next/dynamic"

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../primitive/dropdown-menu"

const UpdateDeposit = dynamic(() => import("@/components/user-action/user-deposit"))
const UserPledgeCollateral = dynamic(
  () => import("@/components/user-action/user-pledge-collateral"),
)

const UserActions = ({ userId }: { userId: string }) => {
  const [openPledgeCollateral, setOpenPledgeCollateral] = useState(false)
  const [openDeposit, setOpenDeposit] = useState(false)

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <div className="cursor-pointer bg-primary text-sm text-black rounded-lg px-2 py-1">
            Actions
          </div>
        </DropdownMenuTrigger>
        <DropdownMenuContent className="text-sm">
          <DropdownMenuItem onClick={() => setOpenPledgeCollateral(true)}>
            User Pledge Collateral
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setOpenDeposit(true)}>
            User Deposit
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {openDeposit && (
        <UpdateDeposit userId={userId} open={openDeposit} setOpen={setOpenDeposit} />
      )}
      {openPledgeCollateral && (
        <UserPledgeCollateral
          userId={userId}
          open={openPledgeCollateral}
          setOpen={setOpenPledgeCollateral}
        />
      )}
    </>
  )
}

export { UserActions }