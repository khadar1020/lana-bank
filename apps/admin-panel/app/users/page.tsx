"use client"
import { gql } from "@apollo/client"

import { useState } from "react"

import { IoEllipsisHorizontal } from "react-icons/io5"

import { toast } from "sonner"

import { PageHeading } from "@/components/page-heading"
import { Button } from "@/components/primitive/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/primitive/table"
import {
  Role,
  useUserAssignRoleMutation,
  useUserRevokeRoleMutation,
  useUsersQuery,
} from "@/lib/graphql/generated"
import CreateUserDialog from "@/components/user/create-user-dialog"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/primitive/dropdown-menu"
import { Badge } from "@/components/primitive/badge"
import { Card, CardContent } from "@/components/primitive/card"

gql`
  query Users {
    users {
      userId
      email
      roles
    }
  }

  mutation UserAssignRole($input: UserAssignRoleInput!) {
    userAssignRole(input: $input) {
      user {
        userId
        email
        roles
      }
    }
  }

  mutation UserRevokeRole($input: UserRevokeRoleInput!) {
    userRevokeRole(input: $input) {
      user {
        userId
        email
        roles
      }
    }
  }
`

function UsersPage() {
  const { data: usersList, refetch } = useUsersQuery()
  const [openCreateUserDialog, setOpenCreateUserDialog] = useState<boolean>(false)

  return (
    <div>
      <CreateUserDialog
        setOpenCreateUserDialog={setOpenCreateUserDialog}
        openCreateUserDialog={openCreateUserDialog}
        refetch={refetch}
      />
      <div className="flex justify-between items-center mb-8">
        <PageHeading className="mb-0">Users</PageHeading>
        <Button onClick={() => setOpenCreateUserDialog(true)} variant="primary">
          Add New User
        </Button>
      </div>

      <Card>
        <CardContent className="pt-6">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-1/5">UserId</TableHead>
                <TableHead className="w-1/5">Email</TableHead>
                <TableHead className="w-2/5">Roles</TableHead>
                <TableHead className="w-1/5 text-left pl-4"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {usersList?.users.map((user) => (
                <TableRow key={user.userId}>
                  <TableCell>{user.userId}</TableCell>
                  <TableCell>{user.email}</TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-2 text-textColor-secondary items-center">
                      {user.roles.length > 0
                        ? user.roles.map((role) => (
                            <Badge variant="secondary" key={role}>
                              {formatRole(role)}
                            </Badge>
                          ))
                        : "No roles Assigned"}
                    </div>
                  </TableCell>
                  <TableCell>
                    <RolesDropDown
                      refetch={refetch}
                      userId={user.userId}
                      roles={user.roles}
                    />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  )
}

export default UsersPage

const RolesDropDown = ({
  userId,
  roles,
  refetch,
}: {
  userId: string
  roles: Role[]
  refetch: () => void
}) => {
  const [checkedRoles, setCheckedRoles] = useState<Role[]>(roles)
  const [assignRole, { loading: assigning, error: assignRoleError }] =
    useUserAssignRoleMutation()
  const [revokeRole, { loading: revoking, error: revokeError }] =
    useUserRevokeRoleMutation()

  const handleRoleChange = async (role: Role) => {
    if (checkedRoles.includes(role)) {
      try {
        await revokeRole({ variables: { input: { id: userId, role } } })
        setCheckedRoles((prev) => prev.filter((r) => r !== role))
        refetch()
        toast.success("Role revoked")
      } catch (err) {
        toast.error(`Failed to revoke role ,${revokeError?.message}`)
      }
    } else {
      try {
        await assignRole({ variables: { input: { id: userId, role } } })
        setCheckedRoles((prev) => [...prev, role])
        refetch()
        toast.success("Role assigned")
      } catch (err) {
        toast.error(`Failed to assign role, ${assignRoleError?.message}`)
      }
    }
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger>
        <Button variant="ghost">
          <IoEllipsisHorizontal className="w-4 h-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuLabel>Roles</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {Object.values(Role).map((role) => (
          <DropdownMenuCheckboxItem
            key={role}
            checked={checkedRoles.includes(role)}
            onCheckedChange={() => handleRoleChange(role)}
            disabled={assigning || revoking}
          >
            {formatRole(role)}
          </DropdownMenuCheckboxItem>
        ))}
        {(assigning || revoking) && <div>Loading...</div>}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

const formatRole = (role: string) => {
  return role
    .toLowerCase()
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ")
}