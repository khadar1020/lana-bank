import React, { useCallback, useEffect, useState } from "react"
import { useDropzone } from "react-dropzone"
import { gql } from "@apollo/client"
import { CgSpinner } from "react-icons/cg"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/primitive/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/primitive/table"

import {
  GetCustomerQuery,
  useCustomerDocumentAttachMutation,
  useDocumentDownloadLinkGenerateMutation,
} from "@/lib/graphql/generated"
import { Button } from "@/components/primitive/button"

type DocumentProps = {
  customer: NonNullable<GetCustomerQuery["customer"]>
  refetch: () => void
}
export const Documents: React.FC<DocumentProps> = ({ customer, refetch }) => {
  return (
    <>
      <CustomerDocuments documents={customer.documents} />
      <AddDocument customer={customer} refetch={refetch} />
    </>
  )
}

gql`
  mutation DocumentDownloadLinkGenerate($input: DocumentDownloadLinksGenerateInput!) {
    documentDownloadLinkGenerate(input: $input) {
      link
    }
  }
`

type CustomerDocumentsProps = {
  documents: NonNullable<GetCustomerQuery["customer"]>["documents"]
}
const CustomerDocuments: React.FC<CustomerDocumentsProps> = ({ documents }) => {
  const [linkLoading, setLinkLoading] = useState<{ [key: string]: boolean }>({})

  const [documentDownloadLinkGenerate] = useDocumentDownloadLinkGenerateMutation()

  const openFile = useCallback(
    async (id: string) => {
      setLinkLoading((prev) => ({ ...prev, [id]: true }))

      const { data } = await documentDownloadLinkGenerate({
        variables: { input: { documentId: id } },
      }).finally(() => setLinkLoading((prev) => ({ ...prev, [id]: false })))

      if (!data?.documentDownloadLinkGenerate?.link) {
        return alert("Failed to generate download link")
      }

      window.open(data.documentDownloadLinkGenerate.link, "_blank")
    },
    [documentDownloadLinkGenerate],
  )

  return (
    <Card className="mt-4">
      <CardHeader>
        <CardTitle>Documents</CardTitle>
      </CardHeader>
      {documents.length === 0 ? (
        <CardContent>No documents found for this customer</CardContent>
      ) : (
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>File Name</TableHead>
                <TableHead></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {documents.map((document) => (
                <TableRow key={document.id}>
                  <TableCell>{document.id}</TableCell>
                  <TableCell>{document.filename}</TableCell>
                  <TableCell>
                    <Button
                      variant="secondary"
                      onClick={() => openFile(document.id)}
                      loading={linkLoading[document.id] || false}
                    >
                      View
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      )}
    </Card>
  )
}

gql`
  mutation CustomerDocumentAttach($file: Upload!, $customerId: UUID!) {
    customerDocumentAttach(input: { file: $file, customerId: $customerId }) {
      document {
        id
        customerId
        filename
      }
    }
  }
`

const AddDocument: React.FC<DocumentProps> = ({ customer, refetch }) => {
  const [customerDocumentAttach, { loading }] = useCustomerDocumentAttachMutation()

  const handleFileUpload = useCallback(
    async (file: File) => {
      await customerDocumentAttach({
        variables: {
          customerId: customer.customerId,
          file,
        },
        onCompleted: () => refetch(),
      })
    },
    [customerDocumentAttach, customer.customerId, refetch],
  )

  const onDrop = useCallback(
    async (acceptedFiles: File[]) => {
      if (acceptedFiles.length > 0) {
        const file = acceptedFiles[0]
        if (file.type === "application/pdf") {
          await handleFileUpload(file)
        } else {
          alert("Please upload only PDF files.")
        }
      }
    },
    [handleFileUpload],
  )

  const { getRootProps, getInputProps } = useDropzone({
    onDrop,
    multiple: false,
  })

  const handlePaste = useCallback(
    (event: ClipboardEvent) => {
      const items = event.clipboardData?.items
      if (items) {
        if (
          items.length === 1 &&
          items[0].kind === "file" &&
          items[0].type === "application/pdf"
        ) {
          const file = items[0].getAsFile()
          if (file) {
            handleFileUpload(file)
          }
        }
      }
    },
    [handleFileUpload],
  )

  useEffect(() => {
    document.addEventListener("paste", handlePaste)
    return () => document.removeEventListener("paste", handlePaste)
  }, [handlePaste])

  return (
    <div {...getRootProps()}>
      <input {...getInputProps()} id="fileInput" disabled={loading} />
      <Card variant="dashed" className="mt-4 cursor-pointer">
        <CardContent className="p-6 flex justify-center items-center w-full h-20">
          {loading ? (
            <CgSpinner className="animate-spin h-5 w-5" />
          ) : (
            <span>
              Click, drag and drop, or paste a PDF file to upload a document for this user
            </span>
          )}
        </CardContent>
      </Card>
    </div>
  )
}